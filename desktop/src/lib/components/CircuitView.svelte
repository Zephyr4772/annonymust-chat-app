<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  
  interface Circuit {
    circuit_id: number;
    hops: string[];
    is_ready: boolean;
    created_at: number;
    last_used: number;
  }
  
  let circuits: Circuit[] = $state([]);
  let isBuilding = $state(false);
  let error = $state('');
  
  onMount(async () => {
    await loadCircuits();
    // Refresh every 3 seconds
    const interval = setInterval(loadCircuits, 3000);
    return () => clearInterval(interval);
  });
  
  async function loadCircuits() {
    try {
      circuits = await invoke<Circuit[]>('get_circuits');
    } catch (err) {
      console.error('Failed to load circuits:', err);
    }
  }
  
  async function buildCircuit() {
    try {
      isBuilding = true;
      error = '';
      
      const circuitId = await invoke<number>('build_circuit');
      console.log('Built circuit:', circuitId);
      
      await loadCircuits();
    } catch (err) {
      error = String(err);
    } finally {
      isBuilding = false;
    }
  }
  
  async function closeCircuit(circuitId: number) {
    try {
      await invoke('close_circuit', { circuitId });
      await loadCircuits();
    } catch (err) {
      error = String(err);
    }
  }
  
  function formatTime(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / 60000);
    
    if (minutes < 1) return 'Just now';
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    return date.toLocaleDateString();
  }
  
  function getRelayColor(index: number): string {
    const colors = [
      'from-blue-400 to-blue-600',      // Guard
      'from-purple-400 to-purple-600',  // Secondary
      'from-pink-400 to-pink-600',      // Middle
      'from-orange-400 to-orange-600',  // Exit
    ];
    return colors[index] || colors[0];
  }
  
  function getRelayRole(index: number): string {
    const roles = ['Guard', 'Secondary', 'Middle', 'Exit'];
    return roles[index] || 'Relay';
  }
</script>

<div class="circuit-view">
  <!-- Header -->
  <div class="header">
    <div class="header-info">
      <span class="material-icons">account_tree</span>
      <div>
        <h2>Onion Circuits</h2>
        <p class="subtitle">{circuits.length} active circuit{circuits.length !== 1 ? 's' : ''}</p>
      </div>
    </div>
    <button class="build-btn" on:click={buildCircuit} disabled={isBuilding}>
      {#if isBuilding}
        <span class="material-icons spinning">sync</span>
        <span>Building...</span>
      {:else}
        <span class="material-icons">add_circle</span>
        <span>Build Circuit</span>
      {/if}
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
  
  <!-- Circuits Container -->
  <div class="circuits-container">
    {#if circuits.length === 0}
      <div class="empty-state">
        <span class="material-icons">router</span>
        <p>No active circuits</p>
        <p class="subtitle">Build a 4-hop circuit for anonymous routing</p>
        <button class="build-empty-btn" on:click={buildCircuit} disabled={isBuilding}>
          <span class="material-icons">add</span>
          Build First Circuit
        </button>
      </div>
    {:else}
      <div class="circuit-list">
        {#each circuits as circuit (circuit.circuit_id)}
          <div class="circuit-card">
            <div class="circuit-header">
              <div class="circuit-id-section">
                <span class="material-icons">vpn_key</span>
                <div>
                  <h3>Circuit #{circuit.circuit_id}</h3>
                  <span class="circuit-time">{formatTime(circuit.created_at)}</span>
                </div>
              </div>
              <div class="circuit-actions">
                {#if circuit.is_ready}
                  <span class="status-badge ready">
                    <span class="material-icons">check_circle</span>
                    Ready
                  </span>
                {:else}
                  <span class="status-badge building">
                    <span class="material-icons spinning">sync</span>
                    Building
                  </span>
                {/if}
                <button class="close-btn" on:click={() => closeCircuit(circuit.circuit_id)}>
                  <span class="material-icons">close</span>
                </button>
              </div>
            </div>
            
            <!-- 4-Hop Visualization -->
            <div class="relay-chain">
              {#each circuit.hops as hop, index (index)}
                <div class="relay-node">
                  <div class="relay-icon bg-gradient-to-br {getRelayColor(index)}">
                    <span class="material-icons">dns</span>
                  </div>
                  <div class="relay-info">
                    <div class="relay-role">{getRelayRole(index)}</div>
                    <div class="relay-address">{hop}</div>
                  </div>
                </div>
                {#if index < circuit.hops.length - 1}
                  <div class="relay-arrow">
                    <span class="material-icons">arrow_forward</span>
                  </div>
                {/if}
              {/each}
            </div>
            
            <!-- Circuit Stats -->
            <div class="circuit-footer">
              <div class="stat">
                <span class="material-icons-outlined">multiple_stop</span>
                <span>{circuit.hops.length} hops</span>
              </div>
              <div class="stat">
                <span class="material-icons-outlined">schedule</span>
                <span>Last used: {formatTime(circuit.last_used)}</span>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .circuit-view {
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
  
  .build-btn, .build-empty-btn {
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
  
  .build-btn:hover:not(:disabled), .build-empty-btn:hover:not(:disabled) {
    background: var(--accent-hover);
    transform: translateY(-1px);
  }
  
  .build-btn:disabled, .build-empty-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .spinning {
    animation: spin 1s linear infinite;
  }
  
  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
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
  
  .circuits-container {
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
  
  .build-empty-btn {
    margin-top: 20px;
  }
  
  .circuit-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  
  .circuit-card {
    background: var(--bg-secondary);
    border-radius: 16px;
    border: 1px solid var(--border-color);
    padding: 20px;
    transition: all 0.2s;
  }
  
  .circuit-card:hover {
    border-color: var(--accent-color);
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }
  
  .circuit-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }
  
  .circuit-id-section {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  
  .circuit-id-section .material-icons {
    font-size: 32px;
    color: var(--accent-color);
  }
  
  .circuit-id-section h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }
  
  .circuit-time {
    font-size: 12px;
    color: var(--text-secondary);
  }
  
  .circuit-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  
  .status-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
  }
  
  .status-badge.ready {
    background: rgba(76, 175, 80, 0.1);
    color: #4caf50;
  }
  
  .status-badge.building {
    background: rgba(33, 150, 243, 0.1);
    color: #2196f3;
  }
  
  .status-badge .material-icons {
    font-size: 16px;
  }
  
  .close-btn {
    padding: 6px;
    background: transparent;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    cursor: pointer;
    color: var(--text-secondary);
    transition: all 0.2s;
  }
  
  .close-btn:hover {
    background: rgba(244, 67, 54, 0.1);
    border-color: #f44336;
    color: #f44336;
  }
  
  .relay-chain {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 20px;
    background: var(--bg-primary);
    border-radius: 12px;
    margin-bottom: 16px;
    overflow-x: auto;
  }
  
  .relay-node {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    min-width: 120px;
  }
  
  .relay-icon {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  }
  
  .relay-icon .material-icons {
    font-size: 28px;
  }
  
  .relay-info {
    text-align: center;
  }
  
  .relay-role {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--accent-color);
    margin-bottom: 4px;
  }
  
  .relay-address {
    font-size: 11px;
    color: var(--text-secondary);
    font-family: 'Courier New', monospace;
  }
  
  .relay-arrow {
    color: var(--text-secondary);
    opacity: 0.5;
  }
  
  .relay-arrow .material-icons {
    font-size: 20px;
  }
  
  .circuit-footer {
    display: flex;
    gap: 24px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  
  .stat {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  
  .stat .material-icons-outlined {
    font-size: 16px;
  }
</style>
