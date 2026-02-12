<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount, onDestroy } from 'svelte';
  
  interface FileTransfer {
    transfer_id: string;
    filename: string;
    file_size: number;
    peer_id: string;
    direction: string;
    status: string;
    progress: number;
    created_at: string;
  }
  
  let transfers: FileTransfer[] = [];
  let isLoading = false;
  let error = '';
  
  onMount(async () => {
    await loadTransfers();
    // Refresh every 2 seconds
    const interval = setInterval(loadTransfers, 2000);
    return () => clearInterval(interval);
  });
  
  async function loadTransfers() {
    try {
      transfers = await invoke<FileTransfer[]>('get_file_transfers');
    } catch (err) {
      console.error('Failed to load transfers:', err);
    }
  }
  
  async function selectFile() {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
      });
      
      if (!selected) return;
      
      // For MVP, we'll need recipient selection
      // For now, use a mock recipient
      const recipientHex = '0'.repeat(64);
      
      isLoading = true;
      error = '';
      
      const transferId = await invoke<string>('send_file', {
        filePath: selected,
        recipientHex,
      });
      
      // Start sending
      await invoke('start_sending', { transferId });
      
      await loadTransfers();
    } catch (err) {
      error = String(err);
    } finally {
      isLoading = false;
    }
  }
  
  async function cancelTransfer(transferId: string) {
    try {
      await invoke('cancel_file_transfer', { transferId });
      await loadTransfers();
    } catch (err) {
      error = String(err);
    }
  }
  
  function formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  }
  
  function formatTime(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / 60000);
    
    if (minutes < 1) return 'Just now';
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    return date.toLocaleDateString();
  }
  
  function getStatusColor(status: string): string {
    switch (status) {
      case 'Completed': return 'text-green-500';
      case 'InProgress': return 'text-blue-500';
      case 'Failed': return 'text-red-500';
      case 'Cancelled': return 'text-gray-400';
      default: return 'text-yellow-500';
    }
  }
  
  function getStatusIcon(status: string): string {
    switch (status) {
      case 'Completed': return 'check_circle';
      case 'InProgress': return 'sync';
      case 'Failed': return 'error';
      case 'Cancelled': return 'cancel';
      default: return 'schedule';
    }
  }
</script>

<div class="file-transfer-view">
  <!-- Header -->
  <div class="header">
    <div class="header-info">
      <span class="material-icons">folder_open</span>
      <div>
        <h2>File Transfers</h2>
        <p class="subtitle">{transfers.length} transfer{transfers.length !== 1 ? 's' : ''}</p>
      </div>
    </div>
    <button class="send-file-btn" on:click={selectFile} disabled={isLoading}>
      <span class="material-icons">upload_file</span>
      <span>Send File</span>
    </button>
  </div>
  
  <!-- Error Banner -->
  {#if error}
    <div class="error-banner">
      <span class="material-icons">error</span>
      <span>{error}</span>
      <button on:click={() => error = ''}>
        <span class="material-icons">close</span>
      </button>
    </div>
  {/if}
  
  <!-- Transfer List -->
  <div class="transfers-container">
    {#if transfers.length === 0}
      <div class="empty-state">
        <span class="material-icons">cloud_upload</span>
        <p>No file transfers yet</p>
        <p class="subtitle">Send a file to get started</p>
      </div>
    {:else}
      <div class="transfer-list">
        {#each transfers as transfer (transfer.transfer_id)}
          <div class="transfer-card">
            <div class="transfer-icon">
              <span class="material-icons">description</span>
            </div>
            
            <div class="transfer-info">
              <div class="transfer-header">
                <h3>{transfer.filename}</h3>
                <span class="file-size">{formatFileSize(transfer.file_size)}</span>
              </div>
              
              <div class="transfer-meta">
                <span class="direction">
                  <span class="material-icons-outlined">
                    {transfer.direction === 'Sending' ? 'arrow_upward' : 'arrow_downward'}
                  </span>
                  {transfer.direction}
                </span>
                <span class="time">{formatTime(transfer.created_at)}</span>
              </div>
              
              <!-- Progress Bar -->
              {#if transfer.status === 'InProgress'}
                <div class="progress-container">
                  <div class="progress-bar">
                    <div class="progress-fill" style="width: {transfer.progress * 100}%"></div>
                  </div>
                  <span class="progress-text">{Math.round(transfer.progress * 100)}%</span>
                </div>
              {/if}
              
              <div class="transfer-footer">
                <span class="status {getStatusColor(transfer.status)}">
                  <span class="material-icons">{getStatusIcon(transfer.status)}</span>
                  {transfer.status}
                </span>
                
                {#if transfer.status === 'InProgress'}
                  <button class="cancel-btn" on:click={() => cancelTransfer(transfer.transfer_id)}>
                    Cancel
                  </button>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .file-transfer-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }
  
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }
  
  .header-info {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  
  .header-info .material-icons {
    font-size: 40px;
    color: var(--accent-color);
  }
  
  .header h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }
  
  .subtitle {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
  }
  
  .send-file-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    background: var(--accent-color);
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    transition: all 0.2s;
  }
  
  .send-file-btn:hover:not(:disabled) {
    background: var(--accent-hover);
    transform: translateY(-1px);
  }
  
  .send-file-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: rgba(244, 67, 54, 0.1);
    border-bottom: 1px solid rgba(244, 67, 54, 0.3);
    color: #f44336;
    font-size: 13px;
  }
  
  .error-banner button {
    margin-left: auto;
    background: none;
    border: none;
    cursor: pointer;
    color: inherit;
    padding: 4px;
  }
  
  .transfers-container {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }
  
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
  
  .empty-state .material-icons {
    font-size: 64px;
    opacity: 0.3;
    margin-bottom: 16px;
  }
  
  .empty-state p {
    margin: 4px 0;
  }
  
  .transfer-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  
  .transfer-card {
    display: flex;
    gap: 16px;
    padding: 16px;
    background: var(--bg-secondary);
    border-radius: 12px;
    border: 1px solid var(--border-color);
    transition: all 0.2s;
  }
  
  .transfer-card:hover {
    border-color: var(--accent-color);
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }
  
  .transfer-icon {
    width: 48px;
    height: 48px;
    background: linear-gradient(135deg, var(--accent-color), var(--accent-hover));
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    flex-shrink: 0;
  }
  
  .transfer-info {
    flex: 1;
    min-width: 0;
  }
  
  .transfer-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 8px;
  }
  
  .transfer-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    truncate: ellipsis;
    overflow: hidden;
    white-space: nowrap;
  }
  
  .file-size {
    font-size: 12px;
    color: var(--text-secondary);
    margin-left: 8px;
    flex-shrink: 0;
  }
  
  .transfer-meta {
    display: flex;
    gap: 16px;
    margin-bottom: 12px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  
  .direction {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  
  .direction .material-icons-outlined {
    font-size: 16px;
  }
  
  .progress-container {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }
  
  .progress-bar {
    flex: 1;
    height: 6px;
    background: var(--bg-primary);
    border-radius: 3px;
    overflow: hidden;
  }
  
  .progress-fill {
    height: 100%;
    background: var(--accent-color);
    transition: width 0.3s ease;
  }
  
  .progress-text {
    font-size: 12px;
    color: var(--text-secondary);
    min-width: 40px;
    text-align: right;
  }
  
  .transfer-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  
  .status {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    font-weight: 500;
  }
  
  .status .material-icons {
    font-size: 16px;
  }
  
  .cancel-btn {
    padding: 6px 12px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s;
  }
  
  .cancel-btn:hover {
    background: var(--bg-primary);
    color: var(--text-primary);
    border-color: var(--accent-color);
  }
</style>
