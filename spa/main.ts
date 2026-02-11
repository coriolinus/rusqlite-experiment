import wasm_init, { Database, TodoList, apply_schema, Item } from "./ffi";


let db: Database | null = null;
let currentList: TodoList | null = null;

const $ = <T extends HTMLElement>(sel: string): T => {
    const el = document.querySelector(sel);
    if (!el) throw new Error(`Missing element: ${sel}`);
    return el as T;
};

const statusEl = $('#status') as HTMLDivElement;
const listsEl = $('#lists') as HTMLUListElement;
const itemsEl = $('#items') as HTMLUListElement;
const currentListTitleEl = $('#current-list-title') as HTMLHeadingElement;
const itemsControlsEl = $('#items-controls') as HTMLDivElement;

const newListTitleInput = $('#new-list-title') as HTMLInputElement;
const createListBtn = $('#create-list') as HTMLButtonElement;
const newItemDescInput = $('#new-item-desc') as HTMLInputElement;
const addItemBtn = $('#add-item') as HTMLButtonElement;
const saveListBtn = $('#save-list') as HTMLButtonElement;
const deleteListBtn = $('#delete-list') as HTMLButtonElement;

function setStatus(text: string): void {
    statusEl.textContent = text;
}

function clearChildren(el: HTMLElement): void {
    while (el.firstChild) el.removeChild(el.firstChild);
}

async function init(): Promise<void> {
    await wasm_init("./ffi_bg.wasm");
    try {
        setStatus('Connecting to database...');
        db = await Database.connect('todo_app');
        await apply_schema(db);
        setStatus('Loading lists...');
        await refreshLists();
        setStatus('Ready');
    } catch (err: unknown) {
        console.error(err);
        setStatus('Error initializing database: ' + (err instanceof Error ? err.message : String(err)));
    }
}

async function refreshLists(): Promise<void> {
    clearChildren(listsEl);
    if (!db) return;
    try {
        const all = await TodoList.list_all(db);
        const entries = Object.entries(all).map(([id, title]) => ({ id: Number(id), title }));
        entries.sort((a, b) => a.id - b.id);
        for (const e of entries) {
            const li = document.createElement('li');
            const left = document.createElement('div');
            left.className = 'item-left';
            const titleSpan = document.createElement('span');
            titleSpan.textContent = e.title;
            const idSpan = document.createElement('span');
            idSpan.className = 'small';
            idSpan.textContent = `#${e.id}`;
            left.appendChild(titleSpan);
            left.appendChild(idSpan);

            const loadBtn = document.createElement('button');
            loadBtn.textContent = 'Load';
            loadBtn.onclick = () => void loadList(e.id);

            li.appendChild(left);
            li.appendChild(loadBtn);
            listsEl.appendChild(li);
        }
    } catch (err: unknown) {
        console.error(err);
        setStatus('Failed to list todo lists: ' + (err instanceof Error ? err.message : String(err)));
    }
}

async function createList(): Promise<void> {
    if (!db) return;
    const title = newListTitleInput.value.trim();
    if (!title) return;
    setStatus('Creating list...');
    try {
        const list = await TodoList.new(db, title);
        await list.save(db);
        newListTitleInput.value = '';
        await refreshLists();
        setStatus('List created');
        await loadList(list.id());
    } catch (err: unknown) {
        console.error(err);
        setStatus('Failed to create list: ' + (err instanceof Error ? err.message : String(err)));
    }
}

async function loadList(id: number): Promise<void> {
    if (!db) return;
    setStatus('Loading list #' + id + '...');
    try {
        if (currentList) {
            try { currentList.free(); } catch { }
            currentList = null;
        }
        currentList = await TodoList.load(db, id);
        renderCurrentList();
        setStatus('List loaded');
    } catch (err: unknown) {
        console.error(err);
        setStatus('Failed to load list: ' + (err instanceof Error ? err.message : String(err)));
    }
}

function renderCurrentList(): void {
    clearChildren(itemsEl);
    if (!currentList) {
        currentListTitleEl.textContent = 'No list loaded';
        itemsControlsEl.classList.add('hidden');
        return;
    }
    currentListTitleEl.textContent = currentList.title();
    itemsControlsEl.classList.remove('hidden');

    const ids = currentList.item_ids();
    for (let i = 0; i < ids.length; i++) {
        const itemId = ids[i];
        const item = currentList.item(itemId);
        if (!item) continue;

        const li = document.createElement('li');

        const left = document.createElement('div');
        left.className = 'item-left';

        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.checked = item.is_completed();
        checkbox.onchange = async () => {
            item.set_is_completed(checkbox.checked);
            try {
                await currentList!.save(db!);
                renderCurrentList();
            } catch (err: unknown) {
                console.error(err);
                setStatus('Failed to save after toggle: ' + (err instanceof Error ? err.message : String(err)));
            }
        };

        const desc = document.createElement('span');
        desc.className = 'item-desc';
        if (item.is_completed()) desc.classList.add('completed');
        desc.textContent = item.description();

        left.appendChild(checkbox);
        left.appendChild(desc);

        const right = document.createElement('div');
        right.style.display = 'flex';
        right.style.gap = '8px';

        const editBtn = document.createElement('button');
        editBtn.textContent = 'Edit';
        editBtn.onclick = () => editItem(item);

        const removeBtn = document.createElement('button');
        removeBtn.textContent = 'Remove';
        removeBtn.className = 'danger';
        removeBtn.onclick = async () => {
            try {
                const ok = await currentList!.remove_item(db!, item.id());
                if (ok) {
                    await currentList!.save(db!);
                    renderCurrentList();
                    setStatus('Item removed');
                } else {
                    setStatus('Item not found');
                }
            } catch (err: unknown) {
                console.error(err);
                setStatus('Failed to remove item: ' + (err instanceof Error ? err.message : String(err)));
            }
        };

        right.appendChild(editBtn);
        right.appendChild(removeBtn);

        li.appendChild(left);
        li.appendChild(right);
        itemsEl.appendChild(li);
    }
}

function editItem(item: Item): void {
    const newDesc = prompt('Edit item description', item.description());
    if (newDesc === null) return;
    item.set_description(newDesc);
    if (!currentList || !db) return;
    currentList.save(db).then(() => renderCurrentList()).catch(err => {
        console.error(err);
        setStatus('Failed to save after edit: ' + (err instanceof Error ? err.message : String(err)));
    });
}

async function addItem(): Promise<void> {
    if (!currentList || !db) {
        alert('Load a list first');
        return;
    }
    const desc = newItemDescInput.value.trim();
    if (!desc) return;
    try {
        await currentList.add_item(db, desc);
        await currentList.save(db);
        newItemDescInput.value = '';
        renderCurrentList();
        setStatus('Item added');
    } catch (err: unknown) {
        console.error(err);
        setStatus('Failed to add item: ' + (err instanceof Error ? err.message : String(err)));
    }
}

async function saveList(): Promise<void> {
    if (!currentList || !db) return;
    try {
        await currentList.save(db);
        setStatus('List saved');
        await refreshLists();
    } catch (err: unknown) {
        console.error(err);
        setStatus('Failed to save list: ' + (err instanceof Error ? err.message : String(err)));
    }
}

async function deleteList(): Promise<void> {
    if (!currentList || !db) return;
    if (!confirm('Delete this list? This cannot be undone.')) return;
    try {
        const ok = await TodoList.delete(db, currentList.id());
        if (ok) {
            setStatus('List deleted');
            try { currentList.free(); } catch { }
            currentList = null;
            renderCurrentList();
            await refreshLists();
        } else {
            setStatus('List not found');
        }
    } catch (err: unknown) {
        console.error(err);
        setStatus('Failed to delete list: ' + (err instanceof Error ? err.message : String(err)));
    }
}

createListBtn.addEventListener('click', () => void createList());
addItemBtn.addEventListener('click', () => void addItem());
saveListBtn.addEventListener('click', () => void saveList());
deleteListBtn.addEventListener('click', () => void deleteList());

window.addEventListener('load', () => {
    void init();
});