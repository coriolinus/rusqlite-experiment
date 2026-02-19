/**
 * Handle lifecycle management for worker-owned objects
 */

import type { Handle } from './types';

/**
 * Generic handle manager for worker-owned instances
 */
export class HandleManager<T> {
    private nextHandle: Handle = 1;
    private instances = new Map<Handle, T>();

    /**
     * Create a new handle for an instance
     */
    create(instance: T): Handle {
        const handle = this.nextHandle++;
        this.instances.set(handle, instance);
        return handle;
    }

    /**
     * Retrieve an instance by handle
     * @throws Error if handle not found
     */
    get(handle: Handle): T {
        const instance = this.instances.get(handle);
        if (!instance) {
            throw new Error(`Invalid handle: ${handle}`);
        }
        return instance;
    }

    /**
     * Check if a handle exists
     */
    has(handle: Handle): boolean {
        return this.instances.has(handle);
    }

    /**
     * Delete an instance by handle
     * @returns true if the handle existed
     */
    delete(handle: Handle): boolean {
        return this.instances.delete(handle);
    }

    /**
     * Clear all instances (for shutdown)
     */
    clear(): void {
        this.instances.clear();
    }

    /**
     * Get count of managed instances
     */
    size(): number {
        return this.instances.size;
    }
}
