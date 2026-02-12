<script lang="ts">
  interface Props {
    selectedContact: string | null;
  }
  
  let { selectedContact }: Props = $props();
  
  // Mock messages (will be replaced with Tauri commands later)
  const messages = [
    { id: 1, sender: 'alice', text: 'Hey, just finished setting up the node. Did you verify the new public keys I sent over earlier?', time: '12:30 PM', isSent: false },
    { id: 2, sender: 'me', text: 'Checking them now. Give me a second to run the checksums.', time: '12:35 PM', isSent: true },
    { id: 3, sender: 'me', text: 'Yes, fingerprints match. Sending the payload now.', time: '12:42 PM', isSent: true },
  ];
</script>

{#if selectedContact}
  <main class="flex-1 flex flex-col relative bg-white/30 backdrop-blur-sm">
    <!-- Header -->
    <header class="h-24 px-8 border-b border-border-subtle bg-white/50 backdrop-blur-md flex items-center justify-between">
      <div class="flex items-center gap-5">
        <!-- Avatar -->
        <div class="relative cursor-pointer">
          <div class="w-12 h-12 rounded-full bg-accent-purple flex items-center justify-center text-gray-500 font-medium text-lg shadow-sm">
            {selectedContact[0].toUpperCase()}
          </div>
          <div class="absolute -bottom-0.5 -right-0.5 bg-white p-0.5 rounded-full">
            <div class="w-2.5 h-2.5 bg-green-400 rounded-full"></div>
          </div>
        </div>
        
        <!-- Name & Status -->
        <div>
          <h1 class="font-medium text-xl text-gray-800 tracking-tight capitalize">{selectedContact}</h1>
          <div class="flex items-center gap-2 mt-0.5">
            <span class="w-1.5 h-1.5 rounded-full bg-green-400"></span>
            <p class="text-xs text-gray-500 font-light">Active on Mesh</p>
          </div>
        </div>
      </div>
      
      <!-- Encryption Badge -->
      <div class="flex items-center gap-2 px-4 py-2 rounded-full bg-white border border-gray-100 shadow-sm">
        <span class="material-icons-outlined text-gray-400 text-lg">verified_user</span>
        <span class="text-xs font-medium text-gray-500 tracking-wide uppercase">Encrypted</span>
      </div>
    </header>
    
    <!-- Messages -->
    <div class="flex-1 overflow-y-auto custom-scrollbar p-8 space-y-8">
      <!-- Date Divider -->
      <div class="flex justify-center">
        <span class="text-[11px] font-medium tracking-widest uppercase text-gray-400 bg-white/50 px-4 py-1.5 rounded-full border border-gray-100/50">
          Today
        </span>
      </div>
      
      <!-- Message List -->
      {#each messages as msg}
        <div class="flex flex-col gap-2 {msg.isSent ? 'items-end ml-auto' : 'items-start'} max-w-[70%]">
          <div class="flex items-end gap-3 group">
            {#if !msg.isSent}
              <div class="w-8 h-8 rounded-full bg-accent-purple flex items-center justify-center text-gray-500 text-sm mb-1">
                {msg.sender[0].toUpperCase()}
              </div>
            {/if}
            
            <div class="{msg.isSent ? 'bg-accent-blue' : 'bg-white border border-gray-50'} p-5 rounded-3xl {msg.isSent ? 'rounded-br-sm' : 'rounded-bl-sm'} shadow-soft text-gray-700 relative">
              <p class="text-[15px] leading-relaxed font-light">{msg.text}</p>
              <div class="absolute -bottom-5 {msg.isSent ? 'right-1' : 'left-1'} flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <span class="text-[10px] text-gray-400">{msg.time}</span>
                {#if msg.isSent}
                  <span class="material-icons-outlined text-[12px] text-gray-400">done_all</span>
                {/if}
              </div>
            </div>
          </div>
        </div>
      {/each}
    </div>
    
    <!-- Input Area -->
    <div class="p-8 pb-10 bg-white/40 backdrop-blur-sm">
      <div class="relative flex items-center gap-4 bg-gray-50/80 p-2 pl-4 rounded-full border border-gray-100 focus-within:ring-2 focus-within:ring-gray-100 focus-within:bg-white transition-all shadow-sm max-w-4xl mx-auto">
        <button class="p-2 text-gray-400 hover:text-gray-600 transition-colors rounded-full hover:bg-gray-100">
          <span class="material-icons-outlined text-[20px]">add</span>
        </button>
        
        <textarea
          placeholder="Write a message..."
          rows="1"
          class="w-full bg-transparent border-none focus:ring-0 text-gray-700 placeholder-gray-400 resize-none py-3 h-[46px] custom-scrollbar leading-normal text-[15px] font-light"
        ></textarea>
        
        <button class="text-gray-400 hover:text-gray-800 p-2 rounded-full transition-all flex items-center justify-center h-[40px] w-[40px]">
          <span class="material-icons-outlined text-[24px] font-light">arrow_forward</span>
        </button>
      </div>
      
      <div class="max-w-4xl mx-auto flex justify-center mt-3">
        <span class="text-[10px] text-gray-300 flex items-center gap-1.5 uppercase tracking-wider font-medium">
          <span class="material-icons-outlined text-[10px]">lock</span>
          End-to-End Encrypted
        </span>
      </div>
    </div>
  </main>
{:else}
  <!-- Empty State -->
  <main class="flex-1 flex flex-col items-center justify-center bg-white/30 backdrop-blur-sm relative">
    <div class="absolute inset-0 z-0 pointer-events-none opacity-[0.03]" style="background-image: radial-gradient(#2b2b2b 1px, transparent 1px); background-size: 32px 32px;"></div>
    
    <div class="relative z-10 flex flex-col items-center text-center max-w-lg p-12">
      <div class="relative w-32 h-32 mb-10 flex items-center justify-center">
        <div class="absolute inset-0 rounded-full border border-gray-200 animate-[spin_12s_linear_infinite]"></div>
        <div class="absolute inset-4 rounded-full border border-gray-200 animate-[spin_8s_linear_infinite_reverse]"></div>
        <div class="relative z-10 bg-white p-6 rounded-full shadow-sm border border-gray-100">
          <span class="material-icons-outlined text-4xl text-gray-400 font-thin">lock</span>
        </div>
      </div>
      
      <h1 class="text-3xl font-medium text-gray-800 mb-4 tracking-wide">SECURE CHANNEL</h1>
      <p class="text-gray-500 font-light text-sm mb-10 leading-loose max-w-sm mx-auto">
        Connection routed through decentralized mesh.<br/>
        Select a conversation to begin.
      </p>
      
      <button class="group flex items-center gap-3 px-8 py-3 bg-white hover:bg-gray-50 text-gray-800 border border-gray-200 rounded hover:shadow-md transition-all duration-300">
        <span class="material-icons-outlined text-[18px] text-gray-400 group-hover:text-gray-600 transition-colors">edit</span>
        <span class="font-light tracking-widest text-xs uppercase">New Message</span>
      </button>
      
      <div class="mt-12 flex items-center gap-2 text-[10px] text-gray-300 uppercase tracking-[0.2em] font-light">
        <span class="material-icons-outlined text-[12px]">shield</span>
        End-to-End Encrypted
      </div>
    </div>
  </main>
{/if}
