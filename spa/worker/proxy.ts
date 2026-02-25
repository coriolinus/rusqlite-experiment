/**
 * Main-thread proxy API - mirrors WASM interface but communicates with worker
 */

import type {
    WorkerRequest,
    WorkerResponse,
    SerializedTodoList,
    SerializedItem,
    Handle,
    MessageId,
} from './types';

// Worker instance
let worker: Worker | null = null;

// Message ID counter
let nextMessageId = 1;

// Pending requests awaiting responses
const pendingRequests = new Map<MessageId, {
    resolve: (value: any) => void;
    reject: (error: Error) => void;
}>();

/**
 * Generate a unique message ID
 */
function generateMessageId(): MessageId {
    return `msg_${nextMessageId++}`;
}

/**
 * Send a request to the worker and await response
 */
async function sendRequest<T>(type: string, payload: unknown): Promise<T> {
    if (!worker) {
        throw new Error('Worker not initialized. Call wasm_init() first.');
    }

    const id = generateMessageId();
    const request: WorkerRequest = { id, type, payload };

    return new Promise<T>((resolve, reject) => {
        pendingRequests.set(id, { resolve, reject });
        worker!.postMessage(request);
    });
}

/**
 * Handle worker responses
 */
function handleWorkerMessage(event: MessageEvent<WorkerResponse>): void {
    const response = event.data;
    const pending = pendingRequests.get(response.id);

    if (!pending) {
        console.error('[PROXY] Received response for unknown request:', response.id);
        return;
    }

    pendingRequests.delete(response.id);

    if (response.success) {
        pending.resolve(response.payload);
    } else {
        // Reconstruct error from serialized form with full error chain
        const buildErrorChain = (err: typeof response.error): { message: string; fullChain: string[] } => {
            if (!err || !err.message) {
                return { message: 'Unknown error', fullChain: ['Unknown error'] };
            }

            const messages: string[] = [err.message];
            let current = err.cause;

            while (current && current.message) {
                messages.push(current.message);
                current = current.cause;
            }

            return {
                message: messages[0], // First/outermost message
                fullChain: messages,
            };
        };

        const errorInfo = buildErrorChain(response.error);
        const error = new Error(errorInfo.message);

        // Store the full chain in a custom property for detailed logging
        (error as any).errorChain = errorInfo.fullChain;

        if (response.error?.stack) {
            error.stack = response.error.stack;
        }

        pending.reject(error);
    }
}

/**
 * Initialize WASM in worker
 */
async function wasm_init(wasmPath: string): Promise<void> {
    if (worker) {
        throw new Error('Worker already initialized');
    }

    // Create worker from the bundled module
    worker = new Worker(
        new URL('./wasm-worker.js', import.meta.url),
        { type: 'module' }
    );

    // Set up message handler
    worker.addEventListener('message', handleWorkerMessage);

    // Initialize WASM in worker
    await sendRequest<void>('init', { wasmPath });

    console.log('[PROXY] WASM initialized in worker');
}

/**
 * Database proxy class
 */
export class Database {
    private handle: Handle;

    private constructor(handle: Handle) {
        this.handle = handle;
    }

    /**
     * Connect to a database
     */
    static async connect(name: string): Promise<Database> {
        const { handle } = await sendRequest<{ handle: Handle }>('Database.connect', { name });
        return new Database(handle);
    }

    /**
     * Decrypt the database with the provided key
     */
    async decrypt_database(passphrase: string): Promise<void> {
        await sendRequest<void>('Database.decrypt_database', {
            handle: this.handle,
            passphrase,
        });
    }

    /**
     * Check if the database is encrypted
     */
    async is_encrypted(): Promise<boolean> {
        return await sendRequest<boolean>('Database.is_encrypted', {
            handle: this.handle,
        });
    }

    /**
     * Set the encryption key for the database
     */
    async set_key(passphrase: string): Promise<void> {
        await sendRequest<void>('Database.set_key', {
            handle: this.handle,
            passphrase,
        });
    }

    /**
     * Get the database name
     */
    async name(): Promise<string> {
        return await sendRequest<string>('Database.name', {
            handle: this.handle,
        });
    }

    /**
     * Read the database file from OPFS
     */
    async readDatabaseFile(): Promise<Uint8Array> {
        return await sendRequest<Uint8Array>('Database.readDatabaseFile', {
            handle: this.handle,
        });
    }

    /**
     * Delete the database from OPFS
     */
    async delete(): Promise<void> {
        await sendRequest<void>('Database.delete', {
            handle: this.handle,
        });
    }

    /**
     * Get the internal handle (for TodoList operations)
     */
    getHandle(): Handle {
        return this.handle;
    }
}

/**
 * Apply schema to the database
 */
export async function apply_schema(database: Database): Promise<void> {
    await sendRequest<void>('apply_schema', {
        dbHandle: database.getHandle(),
    });
}

/**
 * Item class - immutable value object from snapshot
 */
export class Item {
    private data: SerializedItem;

    constructor(data: SerializedItem) {
        this.data = data;
    }

    id(): number {
        return this.data.id;
    }

    list_id(): number {
        return this.data.listId;
    }

    description(): string {
        return this.data.description;
    }

    is_completed(): boolean {
        return this.data.isCompleted;
    }

    created_at(): number {
        return this.data.createdAt;
    }

    free(): void {
        // No-op for proxy items (they're just data)
    }
}

/**
 * TodoList proxy class with snapshot caching
 */
export class TodoList {
    private handle: Handle;
    private snapshot: SerializedTodoList;

    private constructor(handle: Handle, snapshot: SerializedTodoList) {
        this.handle = handle;
        this.snapshot = snapshot;
    }

    /**
     * Get all todo lists with their ids
     */
    static async list_all(database: Database): Promise<[number, string][]> {
        return await sendRequest<[number, string][]>('TodoList.list_all', {
            dbHandle: database.getHandle(),
        });
    }

    /**
     * Create a new todo list
     */
    static async new(database: Database, title: string): Promise<TodoList> {
        const { handle, snapshot } = await sendRequest<{ handle: Handle; snapshot: SerializedTodoList }>(
            'TodoList.new',
            {
                dbHandle: database.getHandle(),
                title,
            }
        );
        return new TodoList(handle, snapshot);
    }

    /**
     * Load a todo list by its id
     */
    static async load(database: Database, id: number): Promise<TodoList> {
        const { handle, snapshot } = await sendRequest<{ handle: Handle; snapshot: SerializedTodoList }>(
            'TodoList.load',
            {
                dbHandle: database.getHandle(),
                id,
            }
        );
        return new TodoList(handle, snapshot);
    }

    /**
     * Delete a todo list by its id
     */
    static async delete(database: Database, id: number): Promise<boolean> {
        return await sendRequest<boolean>('TodoList.delete', {
            dbHandle: database.getHandle(),
            id,
        });
    }

    /**
     * Save the todo list and all its items
     */
    async save(database: Database): Promise<void> {
        const { snapshot } = await sendRequest<{ snapshot: SerializedTodoList }>('TodoList.save', {
            handle: this.handle,
            dbHandle: database.getHandle(),
        });
        this.snapshot = snapshot;
    }

    /**
     * Add an item to this todo list
     */
    async add_item(database: Database, description: string): Promise<number> {
        const { itemId, snapshot } = await sendRequest<{ itemId: number; snapshot: SerializedTodoList }>(
            'TodoList.add_item',
            {
                handle: this.handle,
                dbHandle: database.getHandle(),
                description,
            }
        );
        this.snapshot = snapshot;
        return itemId;
    }

    /**
     * Remove an item from this todo list
     */
    async remove_item(database: Database, item_id: number): Promise<boolean> {
        const { ok, snapshot } = await sendRequest<{ ok: boolean; snapshot: SerializedTodoList }>(
            'TodoList.remove_item',
            {
                handle: this.handle,
                dbHandle: database.getHandle(),
                itemId: item_id,
            }
        );
        this.snapshot = snapshot;
        return ok;
    }

    /**
     * Update an item's description (now async)
     */
    async set_item_description(item_id: number, description: string): Promise<boolean | undefined> {
        const { result, snapshot } = await sendRequest<{
            result: boolean | undefined;
            snapshot: SerializedTodoList;
        }>('TodoList.set_item_description', {
            handle: this.handle,
            itemId: item_id,
            description,
        });
        this.snapshot = snapshot;
        return result;
    }

    /**
     * Update an item's completed status (now async)
     */
    async set_item_completed(item_id: number, is_completed: boolean): Promise<boolean | undefined> {
        const { result, snapshot } = await sendRequest<{
            result: boolean | undefined;
            snapshot: SerializedTodoList;
        }>('TodoList.set_item_completed', {
            handle: this.handle,
            itemId: item_id,
            isCompleted: is_completed,
        });
        this.snapshot = snapshot;
        return result;
    }

    /**
     * Free this todo list instance
     */
    free(): void {
        sendRequest<void>('TodoList.free', {
            handle: this.handle,
        }).catch(err => {
            console.error('[PROXY] Error freeing TodoList:', err);
        });
    }

    // ==================== Synchronous Getters (from snapshot) ====================

    id(): number {
        return this.snapshot.id;
    }

    title(): string {
        return this.snapshot.title;
    }

    set_title(title: string): void {
        this.snapshot.title = title;
    }

    created_at(): number {
        return this.snapshot.createdAt;
    }

    item_ids(): Uint32Array {
        return new Uint32Array(this.snapshot.itemIds);
    }

    item(item_id: number): Item | undefined {
        const itemData = this.snapshot.items[item_id];
        return itemData ? new Item(itemData) : undefined;
    }
}

// Export wasm_init as default to match original API
export default wasm_init;
