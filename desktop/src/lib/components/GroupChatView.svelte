<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  
  export let group: { group_id: string; name: string; member_count: number };
  
  interface GroupMessage {
    group_id: string;
    sender_id: string;
    content: string;
    timestamp: number;
  }
  
  let messages: GroupMessage[] = [];
  let newMessage = '';
  let isLoading = false;
  let error = '';
  let messagesContainer: HTMLElement;
  let unlistenGroupMessage: (() => void) | null = null;
  
  onMount(async () => {
    await loadMessages();
    
    // Listen for new group messages
    unlistenGroupMessage = await listen('group-message-received', (event: any) => {
      if (event.payload === group.group_id) {
        loadMessages();
      }
    });
  });
  
  onDestroy(() => {
    if (unlistenGroupMessage) {
      unlistenGroupMessage();
    }
  });
  
  async function loadMessages() {
    try {
      messages = await invoke<GroupMessage[]>('get_group_messages', {
        groupIdHex: group.group_id
      });
      
      // Scroll to bottom after loading
      setTimeout(scrollToBottom, 100);
    } catch (err) {
      error = String(err);
    }
  }
  
  async function sendMessage() {
    if (!newMessage.trim() || isLoading) return;
    
    const messageContent = newMessage.trim();
    newMessage = '';
    isLoading = true;
    error = '';
    
    try {
      await invoke('send_group_message', {
        groupIdHex: group.group_id,
        content: messageContent
      });
      
      await loadMessages();
    } catch (err) {
      error = String(err);
      newMessage = messageContent; // Restore message on error
    } finally {
      isLoading = false;
    }
  }
  
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }
  
  function scrollToBottom() {
    if (messagesContainer) {
      messagesContainer.scrollTop = messagesContainer.scrollHeight;
    }
  }
  
  function formatTime(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const messageDate = new Date(date.getFullYear(), date.getMonth(), date.getDate());
    
    if (messageDate.getTime() === today.getTime()) {
      return date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
    } else {
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    }
  }
  
  function shortenId(id: string): string {
    return id.substring(0, 8);
  }
</script>

<div class="group-chat-view">
  <!-- Header -->
  <div class="group-header">
    <div class="group-info">
      <div class="group-avatar">
        <span class="material-icons">group</span>
      </div>
      <div>
        <h2>{group.name}</h2>
        <p class="member-count">{group.member_count} member{group.member_count !== 1 ? 's' : ''}</p>
      </div>
    </div>
    <button class="icon-btn" aria-label="Group settings">
      <span class="material-icons">more_vert</span>
    </button>
  </div>
  
  <!-- Messages Area -->
  <div class="messages-container" bind:this={messagesContainer}>
    {#if messages.length === 0}
      <div class="empty-state">
        <span class="material-icons">forum</span>
        <p>No messages yet</p>
        <p class="subtitle">Start the conversation!</p>
      </div>
    {:else}
      {#each messages as message (message.timestamp)}
        <div class="message-bubble">
          <div class="message-header">
            <span class="sender-id">@{shortenId(message.sender_id)}</span>
            <span class="timestamp">{formatTime(message.timestamp)}</span>
          </div>
          <div class="message-content">{message.content}</div>
        </div>
      {/each}
    {/if}
  </div>
  
  <!-- Input Area -->
  <div class="input-area">
    {#if error}
      <div class="error-banner">
        <span class="material-icons">error</span>
        <span>{error}</span>
        <button on:click={() => error = ''}>
          <span class="material-icons">close</span>
        </button>
      </div>
    {/if}
    
    <div class="input-wrapper">
      <textarea
        bind:value={newMessage}
        on:keydown={handleKeydown}
        placeholder="Type a message..."
        disabled={isLoading}
        rows="1"
      />
      <button
        class="send-btn"
        on:click={sendMessage}
        disabled={!newMessage.trim() || isLoading}
        aria-label="Send message"
      >
        <span class="material-icons">send</span>
      </button>
    </div>
  </div>
</div>

<style>
  .group-chat-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }
  
  .group-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }
  
  .group-info {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  
  .group-avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--accent-color), var(--accent-hover));
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
  }
  
  .group-info h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }
  
  .member-count {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
  }
  
  .icon-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 8px;
    color: var(--text-secondary);
    transition: color 0.2s;
  }
  
  .icon-btn:hover {
    color: var(--text-primary);
  }
  
  .messages-container {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  
  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
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
  
  .empty-state .subtitle {
    font-size: 14px;
    opacity: 0.7;
  }
  
  .message-bubble {
    background: var(--bg-secondary);
    border-radius: 12px;
    padding: 12px 16px;
    max-width: 70%;
    align-self: flex-start;
  }
  
  .message-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  
  .sender-id {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent-color);
  }
  
  .timestamp {
    font-size: 11px;
    color: var(--text-secondary);
  }
  
  .message-content {
    font-size: 14px;
    color: var(--text-primary);
    line-height: 1.5;
    word-wrap: break-word;
  }
  
  .input-area {
    border-top: 1px solid var(--border-color);
    background: var(--bg-secondary);
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
  
  .input-wrapper {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    padding: 16px 20px;
  }
  
  .input-wrapper textarea {
    flex: 1;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 24px;
    padding: 12px 20px;
    color: var(--text-primary);
    font-size: 14px;
    font-family: inherit;
    resize: none;
    max-height: 120px;
    transition: border-color 0.2s;
  }
  
  .input-wrapper textarea:focus {
    outline: none;
    border-color: var(--accent-color);
  }
  
  .input-wrapper textarea:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
 .send-btn {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    border: none;
    background: var(--accent-color);
    color: white;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
  }
  
  .send-btn:hover:not(:disabled) {
    background: var(--accent-hover);
    transform: scale(1.05);
  }
  
  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
