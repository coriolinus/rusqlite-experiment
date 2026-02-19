/**
 * Web Worker entry point - owns all WASM instances and handles messages
 */

import type {
    WorkerRequest,
    WorkerResponse,
    SerializedError,
    SerializedTodoList,
    SerializedItem,
    Handle,
} from './types';
import { HandleManager } from './handle-manager';

// Import WASM modules directly
import wasm_init_fn, { Database, TodoList, apply_schema } from '../ffi';

// Handle managers
const databaseHandles = new HandleManager<any>();
const todoListHandles = new HandleManager<any>();

let initialized = false;

/**
 * Serialize an error for transmission to main thread
 */
function serializeError(err: unknown): SerializedError {
    if (err instanceof Error) {
        return {
            message: err.message,
            stack: err.stack,
            cause: err.cause ? serializeError(err.cause) : undefined,
        };
    }
    return {
        message: String(err),
    };
}

/**
 * Serialize a TodoList instance to a snapshot
 */
function serializeTodoList(list: TodoList): SerializedTodoList {
    const id = list.id();
    const title = list.title();
    const createdAt = list.created_at();
    const itemIds = Array.from(list.item_ids());

    const items: Record<number, SerializedItem> = {};
    for (const itemId of itemIds) {
        const item = list.item(itemId);
        if (item) {
            items[itemId] = {
                id: item.id(),
                listId: item.list_id(),
                description: item.description(),
                isCompleted: item.is_completed(),
                createdAt: item.created_at(),
            };
            // Free the item after reading
            item.free();
        }
    }

    return {
        id,
        title,
        createdAt,
        itemIds,
        items,
    };
}

/**
 * Handle incoming messages from main thread
 */
async function handleMessage(request: WorkerRequest): Promise<WorkerResponse> {
    try {
        switch (request.type) {
            case 'init': {
                const { wasmPath } = request.payload as { wasmPath: string };

                // Initialize WASM with the provided path
                await wasm_init_fn(wasmPath);

                initialized = true;
                return {
                    id: request.id,
                    success: true,
                    payload: undefined,
                };
            }

            case 'Database.connect': {
                if (!initialized) throw new Error('WASM not initialized');
                const { name } = request.payload as { name: string };
                const db = await Database.connect(name);
                const handle = databaseHandles.create(db);
                return {
                    id: request.id,
                    success: true,
                    payload: { handle },
                };
            }

            case 'Database.decrypt_database': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle, passphrase } = request.payload as { handle: Handle; passphrase: string };
                const db = databaseHandles.get(handle);
                db.decrypt_database(passphrase);
                return {
                    id: request.id,
                    success: true,
                    payload: undefined,
                };
            }

            case 'Database.is_encrypted': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle } = request.payload as { handle: Handle };
                const db = databaseHandles.get(handle);
                const isEncrypted = await db.is_encrypted();
                return {
                    id: request.id,
                    success: true,
                    payload: isEncrypted,
                };
            }

            case 'Database.set_key': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle, passphrase } = request.payload as { handle: Handle; passphrase: string };
                const db = databaseHandles.get(handle);
                db.set_key(passphrase);
                return {
                    id: request.id,
                    success: true,
                    payload: undefined,
                };
            }

            case 'apply_schema': {
                if (!initialized) throw new Error('WASM not initialized');
                const { dbHandle } = request.payload as { dbHandle: Handle };
                const db = databaseHandles.get(dbHandle);
                await apply_schema(db);
                return {
                    id: request.id,
                    success: true,
                    payload: undefined,
                };
            }

            case 'TodoList.list_all': {
                if (!initialized) throw new Error('WASM not initialized');
                const { dbHandle } = request.payload as { dbHandle: Handle };
                const db = databaseHandles.get(dbHandle);
                const lists = await TodoList.list_all(db);
                return {
                    id: request.id,
                    success: true,
                    payload: lists,
                };
            }

            case 'TodoList.new': {
                if (!initialized) throw new Error('WASM not initialized');
                const { dbHandle, title } = request.payload as { dbHandle: Handle; title: string };
                const db = databaseHandles.get(dbHandle);
                const list = await TodoList.new(db, title);
                const handle = todoListHandles.create(list);
                const snapshot = serializeTodoList(list);
                return {
                    id: request.id,
                    success: true,
                    payload: { handle, snapshot },
                };
            }

            case 'TodoList.load': {
                if (!initialized) throw new Error('WASM not initialized');
                const { dbHandle, id } = request.payload as { dbHandle: Handle; id: number };
                const db = databaseHandles.get(dbHandle);
                const list = await TodoList.load(db, id);
                const handle = todoListHandles.create(list);
                const snapshot = serializeTodoList(list);
                return {
                    id: request.id,
                    success: true,
                    payload: { handle, snapshot },
                };
            }

            case 'TodoList.delete': {
                if (!initialized) throw new Error('WASM not initialized');
                const { dbHandle, id } = request.payload as { dbHandle: Handle; id: number };
                const db = databaseHandles.get(dbHandle);
                const ok = await TodoList.delete(db, id);
                return {
                    id: request.id,
                    success: true,
                    payload: ok,
                };
            }

            case 'TodoList.save': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle, dbHandle } = request.payload as { handle: Handle; dbHandle: Handle };
                const list = todoListHandles.get(handle);
                const db = databaseHandles.get(dbHandle);
                await list.save(db);
                const snapshot = serializeTodoList(list);
                return {
                    id: request.id,
                    success: true,
                    payload: { snapshot },
                };
            }

            case 'TodoList.add_item': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle, dbHandle, description } = request.payload as {
                    handle: Handle;
                    dbHandle: Handle;
                    description: string;
                };
                const list = todoListHandles.get(handle);
                const db = databaseHandles.get(dbHandle);
                const itemId = await list.add_item(db, description);
                const snapshot = serializeTodoList(list);
                return {
                    id: request.id,
                    success: true,
                    payload: { itemId, snapshot },
                };
            }

            case 'TodoList.remove_item': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle, dbHandle, itemId } = request.payload as {
                    handle: Handle;
                    dbHandle: Handle;
                    itemId: number;
                };
                const list = todoListHandles.get(handle);
                const db = databaseHandles.get(dbHandle);
                const ok = await list.remove_item(db, itemId);
                const snapshot = serializeTodoList(list);
                return {
                    id: request.id,
                    success: true,
                    payload: { ok, snapshot },
                };
            }

            case 'TodoList.set_item_description': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle, itemId, description } = request.payload as {
                    handle: Handle;
                    itemId: number;
                    description: string;
                };
                const list = todoListHandles.get(handle);
                const result = list.set_item_description(itemId, description);
                const snapshot = serializeTodoList(list);
                return {
                    id: request.id,
                    success: true,
                    payload: { result, snapshot },
                };
            }

            case 'TodoList.set_item_completed': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle, itemId, isCompleted } = request.payload as {
                    handle: Handle;
                    itemId: number;
                    isCompleted: boolean;
                };
                const list = todoListHandles.get(handle);
                const result = list.set_item_completed(itemId, isCompleted);
                const snapshot = serializeTodoList(list);
                return {
                    id: request.id,
                    success: true,
                    payload: { result, snapshot },
                };
            }

            case 'TodoList.free': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle } = request.payload as { handle: Handle };
                const list = todoListHandles.get(handle);
                list.free();
                todoListHandles.delete(handle);
                return {
                    id: request.id,
                    success: true,
                    payload: undefined,
                };
            }

            default:
                throw new Error(`Unknown message type: ${request.type}`);
        }
    } catch (err) {
        return {
            id: request.id,
            success: false,
            error: serializeError(err),
        };
    }
}

// Listen for messages from main thread
self.addEventListener('message', async (event: MessageEvent<WorkerRequest>) => {
    const response = await handleMessage(event.data);
    self.postMessage(response);
});

// Signal that worker is ready
console.log('[WORKER] WASM worker initialized and ready');
