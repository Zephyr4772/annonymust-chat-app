<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  
  interface Props {
    selectedContact: string | null;
  }
  
  let { selectedContact = $bindable() }: Props = $props();
  
  interface Group {
    group_id: string;
    name: string;
    member_count: number;
  }
 
  let groups: Group[] = $state([]);
  let activeGroupId = $state<string | null>(null);
  
  // Handler callbacks
  let onCreateGroup: (() => void) | null = null;
  let onSelectGroup: ((group: Group) => void) | null = null;
  let onShowFiles: (() => void) | null = null;
  let onShowCircuits: (() => void) | null = null;
  
  // Mock conversation data
  const conversations = [
    { id: 'alice', name: 'Alice', lastMessage: 'Yes, fingerprints match...', time: '12:42', unread: true, online: true },
    { id: 'bob', name: 'Bob', lastMessage: 'Packet routing complete.', time: 'Yesterday', unread: false, online: false },
    { id: 'charlie', name: 'Charlie', lastMessage: 'Attachment', time: 'Mon', unread: false, online: false },
  ];
  
  onMount(async () => {
    await loadGroups();
  });
  
  async function loadGroups() {
    try {
      groups = await invoke<Group[]>('get_groups');
    } catch (err) {
      console.error('Failed to load groups:', err);
      groups = [];
    }
  }
  
  function handleCreateGroup() {
    if (onCreateGroup) onCreateGroup();
  }
  
  function handleSelectGroup(group: Group) {
    activeGroupId = group.group_id;
    if (onSelectGroup) onSelectGroup(group);
  }
  
  function handleShowFiles() {
    if (onShowFiles) onShowFiles();
  }
  
  function handleShowCircuits() {
    if (onShowCircuits) onShowCircuits();
  }
  
  // Expose method to parent
  export function refreshGroups() {
    loadGroups();
  }
  
  export function setGroupHandlers(onCreate: () => void, onSelect: (g: Group) => void) {
    onCreateGroup = onCreate;
    onSelectGroup = onSelect;
  }
  
  export function setFileHandler(onFiles: () => void) {
    onShowFiles = onFiles;
  }
  
  export function setCircuitHandler(onCircuits: () => void) {
    onShowCircuits = onCircuits;
  }
</script>

<aside class="w-80 flex flex-col border-r border-border-subtle bg-white/40 backdrop-blur-sm">
  <!-- Header -->
  <div class="p-6 pb-4">
    <h2 class="text-2xl font-light text-gray-800 mb-4 tracking-tight">Messages</h2>
  </div>
  
  <!-- Groups Section -->
  <div class="px-4 pb-4">
    <div class="flex justify-between items-center mb-3">
      <h3 class="text-xs font-semibold text-gray-500 uppercase tracking-wider">Groups</h3>
      <button 
        class="p-1.5 rounded-lg hover:bg-white/60 transition-colors" 
        onclick={() => {
          console.log('Create group clicked');
          handleCreateGroup();
        }}
      >
        <span class="material-icons-outlined text-gray-600 text-[18px]">add</span>
      </button>
    </div>
    
    {#if groups.length === 0}
      <div class="text-xs text-gray-400 text-center py-2 italic">No groups yet</div>
    {:else}
      <div class="space-y-1">
        {#each groups as group}
          <button
            class="w-full p-3 rounded-xl {activeGroupId === group.group_id ? 'bg-white shadow-soft border border-gray-100' : 'hover:bg-white/60'} cursor-pointer transition-all text-left"
            onclick={() => handleSelectGroup(group)}
          >
            <div class="flex items-center gap-3">
              <div class="w-10 h-10 rounded-full bg-accent-purple flex items-center justify-center text-gray-600 ring-2 ring-white shadow-sm">
                <span class="material-icons-outlined text-[20px]">group</span>
              </div>
              <div class="flex-1 min-w-0">
                <div class="font-medium text-gray-800 text-sm truncate">{group.name}</div>
                <div class="text-xs text-gray-400">{group.member_count} member{group.member_count !== 1 ? 's' : ''}</div>
              </div>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  </div>
  
  <!-- Quick Actions -->
  <div class="px-4 pb-4">
    <h3 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-3">Quick Access</h3>
    <div class="space-y-2">
      <button
        class="w-full p-3 rounded-xl hover:bg-white shadow-soft border border-gray-100 cursor-pointer transition-all text-left flex items-center gap-3"
        onclick={() => {
          console.log('Files clicked');
          if (onShowFiles) onShowFiles();
        }}
      >
        <div class="w-10 h-10 rounded-full bg-blue-100 flex items-center justify-center">
          <span class="material-icons-outlined text-blue-600 text-[20px]">folder</span>
        </div>
        <div>
          <div class="font-medium text-gray-800 text-sm">File Transfers</div>
          <div class="text-xs text-gray-500">Send and receive files</div>
        </div>
      </button>
      
      <button
        class="w-full p-3 rounded-xl hover:bg-white shadow-soft border border-gray-100 cursor-pointer transition-all text-left flex items-center gap-3"
        onclick={() => {
          console.log('Circuits clicked');
          if (onShowCircuits) onShowCircuits();
        }}
      >
        <div class="w-10 h-10 rounded-full bg-purple-100 flex items-center justify-center">
          <span class="material-icons-outlined text-purple-600 text-[20px]">route</span>
        </div>
        <div>
          <div class="font-medium text-gray-800 text-sm">Onion Circuits</div>
          <div class="text-xs text-gray-500">Manage routing circuits</div>
        </div>
      </button>
    </div>
  </div>
  
  <!-- Search -->
  <div class="px-4 pb-4">
    <div class="relative group">
      <span class="material-icons-outlined absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 group-focus-within:text-gray-600 transition-colors text-[20px]">
        search
      </span>
      <input
        type="text"
        placeholder="Search..."
        class="w-full pl-10 pr-4 py-3 rounded-xl bg-white border border-gray-200 focus:border-gray-300 focus:bg-white focus:ring-0 text-sm placeholder-gray-400 text-gray-700 shadow-sm transition-all"
      />
    </div>
  </div>
  
</aside>
