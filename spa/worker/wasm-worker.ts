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
const databaseHandles = new HandleManager<Database>();
const todoListHandles = new HandleManager<TodoList>();

let initialized = false;

/**
 * Serialize an error for transmission to main thread
 */
function serializeError(err: unknown): SerializedError {
    // Handle Map objects
    if (err instanceof Map) {
        if (err.has('msg')) {
            // probably a rust-style error
            const source = err.get('source');
            return {
                message: err.get('msg'),
                cause: source ? serializeError(source) : undefined,
            }
        }

        // fallback to generic map handling
        const entries = Array.from(err.entries()).map(([k, v]) => `${k}: ${String(v)}`);
        return {
            message: `Map error: ${entries.join(', ')}`,
        };
    }

    // Standard JavaScript Error
    if (err instanceof Error) {
        return {
            message: err.message,
            stack: err.stack,
            cause: err.cause ? serializeError(err.cause) : undefined,
        };
    }

    // Handle Rust WASM errors which come as structured objects with 'msg' and 'source' fields
    if (err && typeof err === 'object' && 'msg' in err) {
        const rustErr = err as { msg: string; source?: unknown };
        return {
            message: rustErr.msg,
            cause: rustErr.source ? serializeError(rustErr.source) : undefined,
        };
    }

    // Handle objects with a message property
    if (err && typeof err === 'object' && 'message' in err) {
        const msgErr = err as { message: unknown; cause?: unknown };
        const messageStr = String(msgErr.message);

        // If the message itself is [object ...], try to extract better info
        if (messageStr.startsWith('[object ')) {
            const objStr = extractErrorString(err);
            if (objStr) {
                return { message: objStr };
            }
        }

        return {
            message: messageStr,
            cause: msgErr.cause ? serializeError(msgErr.cause) : undefined,
        };
    }

    // Try to extract useful information from the error object
    if (err && typeof err === 'object') {
        const objStr = extractErrorString(err);
        if (objStr) {
            return { message: objStr };
        }
    }

    // Final fallback - avoid [object ...] strings
    const fallback = String(err);

    if (fallback.startsWith('[object ')) {
        return {
            message: `Unknown error type: ${typeof err}, constructor: ${(err as any)?.constructor?.name || 'unknown'}`,
        };
    }

    return {
        message: fallback,
    };
}

/**
 * Extract a useful error string from an object without serializing non-serializable properties
 */
function extractErrorString(obj: object): string | null {
    // Try toString() first (but not if it returns [object ...])
    try {
        const str = obj.toString();
        if (str && !str.startsWith('[object ')) {
            return str;
        }
    } catch {
        // Continue to other methods
    }

    // Try to build a string from safe properties
    try {
        const parts: string[] = [];
        for (const [key, value] of Object.entries(obj)) {
            // Skip non-serializable types
            if (typeof value === 'function' || value instanceof Map || value instanceof Set) {
                continue;
            }

            try {
                let valueStr: string;
                if (value === null) {
                    valueStr = 'null';
                } else if (value === undefined) {
                    valueStr = 'undefined';
                } else if (typeof value === 'object') {
                    // For objects, try JSON.stringify but with a check
                    const jsonStr = JSON.stringify(value);
                    valueStr = jsonStr.startsWith('[object ') ? String(value) : jsonStr;
                } else {
                    valueStr = String(value);
                }

                parts.push(`${key}: ${valueStr}`);
            } catch {
                // Try a simple String() as last resort
                try {
                    const simpleStr = String(value);
                    if (!simpleStr.startsWith('[object ')) {
                        parts.push(`${key}: ${simpleStr}`);
                    }
                } catch {
                    // Skip this property entirely
                }
            }
        }

        if (parts.length > 0) {
            return parts.join(', ');
        }
    } catch {
        // All extraction methods failed
    }

    return null;
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

            case 'Database.name': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle } = request.payload as { handle: Handle };
                const db = databaseHandles.get(handle);
                const name = db.name();
                return {
                    id: request.id,
                    success: true,
                    payload: name,
                };
            }

            case 'Database.readDatabaseFile': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle } = request.payload as { handle: Handle };
                const db = databaseHandles.get(handle);
                const data = db.export();

                return {
                    id: request.id,
                    success: true,
                    payload: data,
                };
            }

            case 'Database.delete': {
                if (!initialized) throw new Error('WASM not initialized');
                const { handle } = request.payload as { handle: Handle };
                const db = databaseHandles.get(handle);
                await db.delete();
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
                console.log(`[WORKER]: TodoList.list_all -> ${JSON.stringify(lists)}`);
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
