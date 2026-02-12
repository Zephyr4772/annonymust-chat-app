<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { createEventDispatcher } from 'svelte';
  
  export let isOpen = false;
  
  const dispatch = createEventDispatcher();
  
  let groupName = '';
  let isLoading = false;
  let error = '';
  
  async function createGroup() {
    if (!groupName.trim()) {
      error = 'Group name is required';
      return;
    }
    
    isLoading = true;
    error = '';
    
    try {
      const groupId = await invoke<string>('create_group', { name: groupName });
      dispatch('created', { groupId, name: groupName });
      closeModal();
    } catch (err) {
      error = String(err);
    } finally {
      isLoading = false;
    }
  }
  
  function closeModal() {
    groupName = '';
    error = '';
    isOpen = false;
  }
  
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') closeModal();
    if (e.key === 'Enter' && !isLoading) createGroup();
  }
</script>

{#if isOpen}
  <div class="modal-overlay" on:click={closeModal} on:keydown={handleKeydown} role="dialog" tabindex="-1">
    <div class="modal-content" on:click|stopPropagation role="document">
      <div class="modal-header">
        <h2>Create New Group</h2>
        <button class="close-btn" on:click={closeModal} aria-label="Close">
          <span class="material-icons">close</span>
        </button>
      </div>
      
      <div class="modal-body">
        <div class="form-group">
          <label for="group-name">Group Name</label>
          <input
            id="group-name"
            type="text"
            bind:value={groupName}
            placeholder="Enter group name"
            disabled={isLoading}
            autofocus
          />
        </div>
        
        {#if error}
          <div class="error-message">
            <span class="material-icons">error</span>
            {error}
          </div>
        {/if}
      </div>
      
      <div class="modal-footer">
        <button class="btn-secondary" on:click={closeModal} disabled={isLoading}>
          Cancel
        </button>
        <button class="btn-primary" on:click={createGroup} disabled={isLoading}>
          {#if isLoading}
            Creating...
          {:else}
            Create Group
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
  }
  
  .modal-content {
    background: var(--bg-secondary);
    border-radius: 12px;
    width: 90%;
    max-width: 500px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
    border: 1px solid var(--border-color);
  }
  
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border-color);
  }
  
  .modal-header h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    color: var(--text-primary);
  }
  
  .close-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px;
    color: var(--text-secondary);
    transition: color 0.2s;
  }
  
  .close-btn:hover {
    color: var(--text-primary);
  }
  
  .modal-body {
    padding: 24px;
  }
  
  .form-group {
    margin-bottom: 16px;
  }
  
  .form-group label {
    display: block;
    margin-bottom: 8px;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
  }
  
  .form-group input {
    width: 100%;
    padding: 12px 16px;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 14px;
    transition: border-color 0.2s;
  }
  
  .form-group input:focus {
    outline: none;
    border-color: var(--accent-color);
  }
  
  .form-group input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .error-message {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px;
    background: rgba(244, 67, 54, 0.1);
    border: 1px solid rgba(244, 67, 54, 0.3);
    border-radius: 8px;
    color: #f44336;
    font-size: 14px;
  }
  
  .error-message .material-icons {
    font-size: 20px;
  }
  
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 24px;
    border-top: 1px solid var(--border-color);
  }
  
  .btn-primary,
  .btn-secondary {
    padding: 10px 24px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    border: none;
  }
  
  .btn-primary {
    background: var(--accent-color);
    color: white;
  }
  
  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .btn-secondary {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
  }
  
  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-primary);
    color: var(--text-primary);
  }
</style>
