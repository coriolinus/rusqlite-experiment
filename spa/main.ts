import wasm_init, { Database, TodoList, apply_schema } from "./ffi";

/**
 * Application state
 */
class AppState {
    db: Database | null = null;
    currentListId: number | null = null;
    currentList: TodoList | null = null;

    setDatabase(db: Database): void {
        this.db = db;
    }

    async loadList(id: number): Promise<void> {
        if (!this.db) throw new Error("Database not initialized");
        
        // Free the previous list if it exists
        if (this.currentList) {
            try { this.currentList.free(); } catch { /* ignore */ }
        }

        this.currentList = await TodoList.load(this.db, id);
        this.currentListId = id;
    }

    unloadList(): void {
        if (this.currentList) {
            try { this.currentList.free(); } catch { /* ignore */ }
        }
        this.currentList = null;
        this.currentListId = null;
    }

    async saveCurrentList(): Promise<void> {
        if (!this.db || !this.currentList) {
            throw new Error("No list loaded");
        }
        await this.currentList.save(this.db);
    }
}

/**
 * DOM element references
 */
class DOMElements {
    readonly status = this.get<HTMLDivElement>('#status');
    readonly listsEl = this.get<HTMLUListElement>('#lists');
    readonly itemsEl = this.get<HTMLUListElement>('#items');
    readonly currentListTitle = this.get<HTMLHeadingElement>('#current-list-title');
    readonly itemsControls = this.get<HTMLDivElement>('#items-controls');
    readonly newListTitleInput = this.get<HTMLInputElement>('#new-list-title');
    readonly createListBtn = this.get<HTMLButtonElement>('#create-list');
    readonly newItemDescInput = this.get<HTMLInputElement>('#new-item-desc');
    readonly addItemBtn = this.get<HTMLButtonElement>('#add-item');
    readonly saveListBtn = this.get<HTMLButtonElement>('#save-list');
    readonly deleteListBtn = this.get<HTMLButtonElement>('#delete-list');
    readonly downloadDbBtn = this.get<HTMLButtonElement>('#download-db');

    private get<T extends HTMLElement>(selector: string): T {
        const el = document.querySelector(selector);
        if (!el) throw new Error(`Missing element: ${selector}`);
        return el as T;
    }

    clearChildren(el: HTMLElement): void {
        while (el.firstChild) {
            el.removeChild(el.firstChild);
        }
    }
}

/**
 * Main application controller
 */
class TodoApp {
    private state = new AppState();
    private dom = new DOMElements();

    async init(): Promise<void> {
        await wasm_init("./ffi_bg.wasm");
        this.setStatus('Connecting to database...');
        
        const db = await Database.connect('todo_app');
        await apply_schema(db);
        this.state.setDatabase(db);

        this.setStatus('Loading lists...');
        await this.renderLists();
        
        this.setStatus('Ready');
        this.attachEventListeners();
    }

    private setStatus(text: string): void {
        this.dom.status.textContent = text;
    }

    private attachEventListeners(): void {
        this.dom.createListBtn.addEventListener('click', () => this.handleCreateList());
        this.dom.addItemBtn.addEventListener('click', () => this.handleAddItem());
        this.dom.saveListBtn.addEventListener('click', () => this.handleSaveList());
        this.dom.deleteListBtn.addEventListener('click', () => this.handleDeleteList());
        this.dom.downloadDbBtn.addEventListener('click', () => this.handleDownloadDatabase());
    }

    // ==================== List Management ====================

    private async renderLists(): Promise<void> {
        this.dom.clearChildren(this.dom.listsEl);
        
        if (!this.state.db) return;

        const all = await TodoList.list_all(this.state.db);
        const entries = all.map(([id, title]) => ({ id: Number(id), title }));
        entries.sort((a, b) => a.id - b.id);

        for (const entry of entries) {
            const li = this.createListElement(entry);
            this.dom.listsEl.appendChild(li);
        }
    }

    private createListElement(entry: { id: number; title: string }): HTMLLIElement {
        const li = document.createElement('li');

        const left = document.createElement('div');
        left.className = 'item-left';

        const titleSpan = document.createElement('span');
        titleSpan.textContent = entry.title;

        const idSpan = document.createElement('span');
        idSpan.className = 'small';
        idSpan.textContent = `#${entry.id}`;

        left.appendChild(titleSpan);
        left.appendChild(idSpan);

        const loadBtn = document.createElement('button');
        loadBtn.textContent = 'Load';
        loadBtn.onclick = () => this.handleLoadList(entry.id);

        li.appendChild(left);
        li.appendChild(loadBtn);

        return li;
    }

    private async handleCreateList(): Promise<void> {
        if (!this.state.db) return;

        const title = this.dom.newListTitleInput.value.trim();
        if (!title) return;

        try {
            this.setStatus('Creating list...');
            const list = await TodoList.new(this.state.db, title);
            await list.save(this.state.db);
            
            this.dom.newListTitleInput.value = '';
            await this.renderLists();
            
            this.setStatus('List created');
            await this.handleLoadList(list.id());
        } catch (err) {
            console.error(err);
            this.setStatus('Failed to create list: ' + this.getErrorMessage(err));
        }
    }

    private async handleLoadList(id: number): Promise<void> {
        try {
            this.setStatus(`Loading list #${id}...`);
            await this.state.loadList(id);
            this.renderItems();
            this.setStatus('List loaded');
        } catch (err) {
            console.error(err);
            this.setStatus('Failed to load list: ' + this.getErrorMessage(err));
        }
    }

    private async handleDeleteList(): Promise<void> {
        if (!this.state.db || !this.state.currentList) return;
        if (!confirm('Delete this list? This cannot be undone.')) return;

        try {
            const id = this.state.currentList.id();
            const ok = await TodoList.delete(this.state.db, id);
            
            if (ok) {
                this.setStatus('List deleted');
                this.state.unloadList();
                this.renderItems();
                await this.renderLists();
            } else {
                this.setStatus('List not found');
            }
        } catch (err) {
            console.error(err);
            this.setStatus('Failed to delete list: ' + this.getErrorMessage(err));
        }
    }

    private async handleSaveList(): Promise<void> {
        try {
            this.setStatus('Saving list...');
            await this.state.saveCurrentList();
            this.setStatus('List saved');
            await this.renderLists();
        } catch (err) {
            console.error(err);
            this.setStatus('Failed to save list: ' + this.getErrorMessage(err));
        }
    }

    // ==================== Item Management ====================

    private renderItems(): void {
        this.dom.clearChildren(this.dom.itemsEl);

        if (!this.state.currentList) {
            this.dom.currentListTitle.textContent = 'No list loaded';
            this.dom.itemsControls.classList.add('hidden');
            return;
        }

        this.dom.currentListTitle.textContent = this.state.currentList.title();
        this.dom.itemsControls.classList.remove('hidden');

        const itemIds = this.state.currentList.item_ids();
        for (let i = 0; i < itemIds.length; i++) {
            const itemId = itemIds[i];
            const itemEl = this.createItemElement(itemId);
            if (itemEl) {
                this.dom.itemsEl.appendChild(itemEl);
            }
        }
    }

    private createItemElement(itemId: number): HTMLLIElement | null {
        if (!this.state.currentList) return null;
        
        const item = this.state.currentList.item(itemId);
        if (!item) return null;

        const li = document.createElement('li');

        // Left side: checkbox and description
        const left = document.createElement('div');
        left.className = 'item-left';

        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.checked = item.is_completed();
        checkbox.onchange = () => this.handleToggleItem(itemId, checkbox.checked);

        const desc = document.createElement('span');
        desc.className = 'item-desc';
        if (item.is_completed()) {
            desc.classList.add('completed');
        }
        desc.textContent = item.description();

        left.appendChild(checkbox);
        left.appendChild(desc);

        // Right side: buttons
        const right = document.createElement('div');
        right.style.display = 'flex';
        right.style.gap = '8px';

        const editBtn = document.createElement('button');
        editBtn.textContent = 'Edit';
        editBtn.onclick = () => this.handleEditItem(itemId);

        const removeBtn = document.createElement('button');
        removeBtn.textContent = 'Remove';
        removeBtn.className = 'danger';
        removeBtn.onclick = () => this.handleRemoveItem(itemId);

        right.appendChild(editBtn);
        right.appendChild(removeBtn);

        li.appendChild(left);
        li.appendChild(right);

        return li;
    }

    private async handleToggleItem(itemId: number, checked: boolean): Promise<void> {
        if (!this.state.currentList || !this.state.db) return;

        try {
            // Use the new set_item_completed method to modify the item directly in the list
            const result = this.state.currentList.set_item_completed(itemId, checked);
            
            if (result === undefined) {
                this.setStatus('Item not found');
                return;
            }

            // Save to database
            await this.state.saveCurrentList();
            
            // Re-render to show updated state
            this.renderItems();
        } catch (err) {
            console.error(err);
            this.setStatus('Failed to toggle item: ' + this.getErrorMessage(err));
        }
    }

    private async handleEditItem(itemId: number): Promise<void> {
        if (!this.state.currentList || !this.state.db) return;

        const item = this.state.currentList.item(itemId);
        if (!item) return;

        const newDesc = prompt('Edit item description', item.description());
        if (newDesc === null) return;

        try {
            // Use the new set_item_description method to modify the item directly in the list
            const result = this.state.currentList.set_item_description(itemId, newDesc);
            
            if (result === undefined) {
                this.setStatus('Item not found');
                return;
            }

            await this.state.saveCurrentList();
            this.renderItems();
            this.setStatus('Item updated');
        } catch (err) {
            console.error(err);
            this.setStatus('Failed to edit item: ' + this.getErrorMessage(err));
        }
    }

    private async handleRemoveItem(itemId: number): Promise<void> {
        if (!this.state.currentList || !this.state.db) return;

        try {
            const ok = await this.state.currentList.remove_item(this.state.db, itemId);
            if (ok) {
                await this.state.saveCurrentList();
                this.renderItems();
                this.setStatus('Item removed');
            } else {
                this.setStatus('Item not found');
            }
        } catch (err) {
            console.error(err);
            this.setStatus('Failed to remove item: ' + this.getErrorMessage(err));
        }
    }

    private async handleAddItem(): Promise<void> {
        if (!this.state.currentList || !this.state.db) {
            alert('Load a list first');
            return;
        }

        const desc = this.dom.newItemDescInput.value.trim();
        if (!desc) return;

        try {
            this.setStatus('Adding item...');
            await this.state.currentList.add_item(this.state.db, desc);
            await this.state.saveCurrentList();
            
            this.dom.newItemDescInput.value = '';
            this.renderItems();
            this.setStatus('Item added');
        } catch (err) {
            console.error(err);
            this.setStatus('Failed to add item: ' + this.getErrorMessage(err));
        }
    }

    // ==================== Utilities ====================

    private getErrorMessage(err: unknown): string {
        return err instanceof Error ? err.message : String(err);
    }

    private async handleDownloadDatabase(): Promise<void> {
        try {
            this.setStatus('Preparing database download...');
            
            // Open the IndexedDB database
            const dbName = 'relaxed-idb';
            const storeName = 'blocks';
            
            const db = await new Promise<IDBDatabase>((resolve, reject) => {
                const request = indexedDB.open(dbName);
                request.onsuccess = () => resolve(request.result);
                request.onerror = () => reject(request.error);
            });

            // Get all blocks from the object store
            const transaction = db.transaction(storeName, 'readonly');
            const store = transaction.objectStore(storeName);
            
            const blocks = await new Promise<any[]>((resolve, reject) => {
                const request = store.getAll();
                request.onsuccess = () => resolve(request.result);
                request.onerror = () => reject(request.error);
            });

            // Filter blocks that start with 'todo_app' and sort by offset
            const todoBlocks = blocks
                .filter(block => block.path === 'todo_app')
                .sort((a, b) => a.offset - b.offset);

            if (todoBlocks.length === 0) {
                this.setStatus('No database blocks found');
                alert('No database data found to download');
                return;
            }

            // Concatenate all the data bytes
            const totalSize = todoBlocks.reduce((sum, block) => {
                return sum + Object.keys(block.data).length;
            }, 0);

            const dbBytes = new Uint8Array(totalSize);
            let position = 0;

            for (const block of todoBlocks) {
                const dataObj = block.data;
                const dataLength = Object.keys(dataObj).length;
                
                for (let i = 0; i < dataLength; i++) {
                    dbBytes[position++] = dataObj[i];
                }
            }

            // Create a blob and download it
            const blob = new Blob([dbBytes], { type: 'application/x-sqlite3' });
            const url = URL.createObjectURL(blob);
            
            const link = document.createElement('a');
            link.href = url;
            link.download = 'todo-list.sqlite';
            link.click();
            
            URL.revokeObjectURL(url);
            
            this.setStatus('Database downloaded');
        } catch (err) {
            console.error('Failed to download database:', err);
            this.setStatus('Failed to download database: ' + this.getErrorMessage(err));
        }
    }
}

// Initialize the app
const app = new TodoApp();

window.addEventListener('load', async () => {
    try {
        await app.init();
    } catch (err) {
        console.error('Failed to initialize app:', err);
        document.querySelector('#status')!.textContent = 
            'Failed to initialize: ' + (err instanceof Error ? err.message : String(err));
    }
});
