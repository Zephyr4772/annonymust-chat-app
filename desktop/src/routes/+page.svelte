<script lang="ts">
  import Sidebar from '$lib/components/Sidebar.svelte';
  import ChatArea from '$lib/components/ChatArea.svelte';
  import GroupCreateModal from '$lib/components/GroupCreateModal.svelte';
  import GroupChatView from '$lib/components/GroupChatView.svelte';
  import FileTransferView from '$lib/components/FileTransferView.svelte';
  import CircuitView from '$lib/components/CircuitView.svelte';
  
  interface Group {
    group_id: string;
    name: string;
    member_count: number;
  }
  
  let selectedContact = $state<string | null>(null);
  let isCreateModalOpen = $state(false);
  let currentView = $state<'direct' | 'group' | 'files' | 'circuits'>('direct');
  let selectedGroup = $state<Group | null>(null);
  let sidebarRef = $state<any>(null);
  
  function handleCreateGroup() {
    isCreateModalOpen = true;
  }
  
  function handleGroupCreated() {
    if (sidebarRef) {
      sidebarRef.refreshGroups();
    }
  }
  
  function handleSelectGroup(group: Group) {
    currentView = 'group';
    selectedGroup = group;
    selectedContact = null;
  }
  
  function handleShowFiles() {
    currentView = 'files';
    selectedGroup = null;
    selectedContact = null;
  }
  
  function handleShowCircuits() {
    currentView = 'circuits';
    selectedGroup = null;
    selectedContact = null;
  }
  
  $effect(() => {
    if (sidebarRef) {
      sidebarRef.setGroupHandlers(handleCreateGroup, handleSelectGroup);
      sidebarRef.setFileHandler(handleShowFiles);
      sidebarRef.setCircuitHandler(handleShowCircuits);
    }
  });
</script>

<div class="flex h-screen bg-gradient-to-br from-gray-50 to-gray-100">
  <Sidebar bind:selectedContact bind:this={sidebarRef} />
  
  {#if currentView === 'circuits'}
    <CircuitView />
  {:else if currentView === 'files'}
    <FileTransferView />
  {:else if currentView === 'group' && selectedGroup}
    <GroupChatView group={selectedGroup} />
  {:else if selectedContact}
    <ChatArea {selectedContact} />
  {:else}
    <!-- Empty state -->
    <div class="flex-1 flex items-center justify-center bg-white/20 backdrop-blur-sm">
      <div class="text-center">
        <span class="material-icons-outlined text-gray-300 text-[80px] mb-4">chat_bubble_outline</span>
        <h2 class="text-2xl font-light text-gray-400 mb-2">No conversation selected</h2>
        <p class="text-gray-400">Choose a contact, group, view files, or manage circuits</p>
      </div>
    </div>
  {/if}
  
  <GroupCreateModal bind:isOpen={isCreateModalOpen} on:created={handleGroupCreated} />
</div>
