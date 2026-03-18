import * as Y from 'yjs';

import type { PeerPresence, SessionProfile } from '$lib/types';

type JoinedMessage = {
	type: 'joined';
	payload: {
		documentId: string;
		serverTime: string;
		peers: PeerPresence[];
	};
};

type SyncInitMessage = {
	type: 'sync_init';
	payload: {
		update: string;
	};
};

type SyncUpdateMessage = {
	type: 'sync_update';
	payload: {
		clientId: string;
		update: string;
	};
};

type PresenceSnapshotMessage = {
	type: 'presence_snapshot';
	payload: {
		peers: PeerPresence[];
	};
};

type ServerMessage = JoinedMessage | SyncInitMessage | SyncUpdateMessage | PresenceSnapshotMessage;

type ConnectionCallbacks = {
	onConnectionState: (state: 'Connecting' | 'Connected' | 'Offline') => void;
	onPresence: (peers: PeerPresence[]) => void;
	onInitialSync: () => void;
};

export class RealtimeClient {
	private socket: WebSocket | null = null;
	private disposed = false;
	private heartbeatTimer: number | null = null;
	private reconnectTimer: number | null = null;
	private didResolveInitialConnect = false;
	private initialConnectSettled = false;
	private syncReady = false;
	private hasPendingLocalChanges = false;

	constructor(
		private readonly documentId: string,
		private readonly session: SessionProfile,
		private readonly ydoc: Y.Doc,
		private readonly callbacks: ConnectionCallbacks
	) {
		this.ydoc.on('update', (update: Uint8Array, origin: unknown) => {
			if (origin === this) {
				return;
			}

			if (!this.syncReady) {
				this.hasPendingLocalChanges = true;
				return;
			}

			if (this.socket?.readyState !== WebSocket.OPEN) {
				return;
			}

			this.send({
				type: 'sync_update',
				payload: {
					clientId: this.session.clientId,
					update: encodeUpdate(update)
				}
			});
		});
	}

	connect(): Promise<void> {
		this.callbacks.onConnectionState('Connecting');
		this.syncReady = false;
		this.initialConnectSettled = false;

		return new Promise((resolve, reject) => {
			const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
			const socket = new WebSocket(`${protocol}//${window.location.host}/api/documents/${this.documentId}/live`);
			this.socket = socket;

			socket.addEventListener('open', () => {
				this.callbacks.onConnectionState('Connected');
				this.send({
					type: 'join',
					payload: {
						clientId: this.session.clientId,
						name: this.session.name,
						color: this.session.color
					}
				});
				this.startHeartbeat();
			});

			socket.addEventListener('message', (event) => {
				const message = JSON.parse(event.data) as ServerMessage;
				this.handleMessage(message, resolve);
			});

			socket.addEventListener('error', () => {
				if (!this.initialConnectSettled && !this.disposed) {
					this.initialConnectSettled = true;
					reject(new Error('Failed to connect to document realtime service'));
				}
			});

			socket.addEventListener('close', () => {
				this.callbacks.onConnectionState('Offline');
				this.stopHeartbeat();
				if (!this.initialConnectSettled && !this.disposed) {
					this.initialConnectSettled = true;
					reject(new Error('Failed to open document realtime session'));
					return;
				}
				if (!this.disposed) {
					this.reconnectTimer = window.setTimeout(() => this.connect(), 1500);
				}
			});
		});
	}

	disconnect() {
		this.disposed = true;
		this.stopHeartbeat();
		if (this.reconnectTimer) {
			window.clearTimeout(this.reconnectTimer);
		}
		this.socket?.close();
	}

	updatePresence(anchor: number, head: number) {
		this.send({
			type: 'presence_update',
			payload: {
				clientId: this.session.clientId,
				anchor,
				head
			}
		});
	}

	private send(payload: unknown) {
		if (this.socket?.readyState === WebSocket.OPEN) {
			this.socket.send(JSON.stringify(payload));
		}
	}

	private startHeartbeat() {
		this.stopHeartbeat();
		this.heartbeatTimer = window.setInterval(() => {
			this.send({
				type: 'heartbeat',
				payload: {
					clientId: this.session.clientId
				}
			});
		}, 10000);
	}

	private stopHeartbeat() {
		if (this.heartbeatTimer) {
			window.clearInterval(this.heartbeatTimer);
			this.heartbeatTimer = null;
		}
	}

	private handleMessage(message: ServerMessage, resolve: () => void) {
		if (message.type === 'sync_init') {
			if (message.payload.update) {
				Y.applyUpdate(this.ydoc, decodeUpdate(message.payload.update), this);
			}

			this.syncReady = true;

			if (this.hasPendingLocalChanges) {
				this.hasPendingLocalChanges = false;
				this.send({
					type: 'sync_update',
					payload: {
						clientId: this.session.clientId,
						update: encodeUpdate(Y.encodeStateAsUpdate(this.ydoc))
					}
				});
			}

			this.callbacks.onInitialSync();
			if (!this.didResolveInitialConnect) {
				this.didResolveInitialConnect = true;
				this.initialConnectSettled = true;
				resolve();
			}
			return;
		}

		if (message.type === 'sync_update') {
			if (message.payload.clientId !== this.session.clientId) {
				Y.applyUpdate(this.ydoc, decodeUpdate(message.payload.update), this);
			}
			return;
		}

		if (message.type === 'presence_snapshot' || message.type === 'joined') {
			this.callbacks.onPresence(message.payload.peers ?? []);
		}
	}
}

function encodeUpdate(update: Uint8Array): string {
	let binary = '';
	for (const byte of update) {
		binary += String.fromCharCode(byte);
	}
	return btoa(binary);
}

function decodeUpdate(value: string): Uint8Array {
	const binary = atob(value);
	const output = new Uint8Array(binary.length);
	for (let index = 0; index < binary.length; index += 1) {
		output[index] = binary.charCodeAt(index);
	}
	return output;
}