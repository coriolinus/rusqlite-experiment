/**
 * Shared TypeScript types for worker message protocol
 */

// ==================== Handle Types ====================

/**
 * Numeric identifier for worker-owned objects
 */
export type Handle = number;

/**
 * Unique identifier for request/response matching
 */
export type MessageId = string;

// ==================== Serialization Types ====================

/**
 * Serialized TodoList snapshot for main thread access
 */
export interface SerializedTodoList {
    id: number;
    title: string;
    createdAt: number;
    itemIds: number[];
    items: Record<number, SerializedItem>;
}

/**
 * Serialized Item snapshot
 */
export interface SerializedItem {
    id: number;
    listId: number;
    description: string;
    isCompleted: boolean;
    createdAt: number;
}

/**
 * Serialized error with stack trace
 */
export interface SerializedError {
    message: string;
    stack?: string;
    cause?: SerializedError;
}

// ==================== Message Protocol ====================

/**
 * Base request structure sent from main thread to worker
 */
export interface WorkerRequest {
    id: MessageId;
    type: string;
    payload: unknown;
}

/**
 * Base response structure sent from worker to main thread
 */
export interface WorkerResponse {
    id: MessageId;
    success: boolean;
    payload?: unknown;
    error?: SerializedError;
}

// ==================== Specific Request/Response Types ====================

// --- Initialization ---

export interface InitRequest {
    type: 'init';
    payload: {
        wasmPath: string;
    };
}

export interface InitResponse {
    success: true;
    payload: void;
}

// --- Database Operations ---

export interface DatabaseConnectRequest {
    type: 'Database.connect';
    payload: {
        name: string;
    };
}

export interface DatabaseConnectResponse {
    success: true;
    payload: {
        handle: Handle;
    };
}

export interface DatabaseDecryptRequest {
    type: 'Database.decrypt_database';
    payload: {
        handle: Handle;
        passphrase: string;
    };
}

export interface DatabaseDecryptResponse {
    success: true;
    payload: void;
}

export interface DatabaseIsEncryptedRequest {
    type: 'Database.is_encrypted';
    payload: {
        handle: Handle;
    };
}

export interface DatabaseIsEncryptedResponse {
    success: true;
    payload: boolean;
}

export interface DatabaseSetKeyRequest {
    type: 'Database.set_key';
    payload: {
        handle: Handle;
        passphrase: string;
    };
}

export interface DatabaseSetKeyResponse {
    success: true;
    payload: void;
}

// --- Schema Operations ---

export interface ApplySchemaRequest {
    type: 'apply_schema';
    payload: {
        dbHandle: Handle;
    };
}

export interface ApplySchemaResponse {
    success: true;
    payload: void;
}

// --- TodoList Static Operations ---

export interface TodoListListAllRequest {
    type: 'TodoList.list_all';
    payload: {
        dbHandle: Handle;
    };
}

export interface TodoListListAllResponse {
    success: true;
    payload: [number, string][];
}

export interface TodoListNewRequest {
    type: 'TodoList.new';
    payload: {
        dbHandle: Handle;
        title: string;
    };
}

export interface TodoListNewResponse {
    success: true;
    payload: {
        handle: Handle;
        snapshot: SerializedTodoList;
    };
}

export interface TodoListLoadRequest {
    type: 'TodoList.load';
    payload: {
        dbHandle: Handle;
        id: number;
    };
}

export interface TodoListLoadResponse {
    success: true;
    payload: {
        handle: Handle;
        snapshot: SerializedTodoList;
    };
}

export interface TodoListDeleteRequest {
    type: 'TodoList.delete';
    payload: {
        dbHandle: Handle;
        id: number;
    };
}

export interface TodoListDeleteResponse {
    success: true;
    payload: boolean;
}

// --- TodoList Instance Operations ---

export interface TodoListSaveRequest {
    type: 'TodoList.save';
    payload: {
        handle: Handle;
        dbHandle: Handle;
    };
}

export interface TodoListSaveResponse {
    success: true;
    payload: {
        snapshot: SerializedTodoList;
    };
}

export interface TodoListAddItemRequest {
    type: 'TodoList.add_item';
    payload: {
        handle: Handle;
        dbHandle: Handle;
        description: string;
    };
}

export interface TodoListAddItemResponse {
    success: true;
    payload: {
        itemId: number;
        snapshot: SerializedTodoList;
    };
}

export interface TodoListRemoveItemRequest {
    type: 'TodoList.remove_item';
    payload: {
        handle: Handle;
        dbHandle: Handle;
        itemId: number;
    };
}

export interface TodoListRemoveItemResponse {
    success: true;
    payload: {
        ok: boolean;
        snapshot: SerializedTodoList;
    };
}

export interface TodoListSetItemDescriptionRequest {
    type: 'TodoList.set_item_description';
    payload: {
        handle: Handle;
        itemId: number;
        description: string;
    };
}

export interface TodoListSetItemDescriptionResponse {
    success: true;
    payload: {
        result: boolean | undefined;
        snapshot: SerializedTodoList;
    };
}

export interface TodoListSetItemCompletedRequest {
    type: 'TodoList.set_item_completed';
    payload: {
        handle: Handle;
        itemId: number;
        isCompleted: boolean;
    };
}

export interface TodoListSetItemCompletedResponse {
    success: true;
    payload: {
        result: boolean | undefined;
        snapshot: SerializedTodoList;
    };
}

export interface TodoListFreeRequest {
    type: 'TodoList.free';
    payload: {
        handle: Handle;
    };
}

export interface TodoListFreeResponse {
    success: true;
    payload: void;
}

// --- Getters (served from snapshot, but included for completeness) ---

export interface TodoListIdRequest {
    type: 'TodoList.id';
    payload: {
        handle: Handle;
    };
}

export interface TodoListTitleRequest {
    type: 'TodoList.title';
    payload: {
        handle: Handle;
    };
}

export interface TodoListCreatedAtRequest {
    type: 'TodoList.created_at';
    payload: {
        handle: Handle;
    };
}

export interface TodoListItemIdsRequest {
    type: 'TodoList.item_ids';
    payload: {
        handle: Handle;
    };
}

export interface TodoListItemRequest {
    type: 'TodoList.item';
    payload: {
        handle: Handle;
        itemId: number;
    };
}
