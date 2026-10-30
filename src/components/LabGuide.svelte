<script lang="ts">
  import { Search, BookOpen, Images, Globe, Check } from "lucide-svelte";

  let {
    kind,
    onStep,
    completedStages = [],
  }: {
    kind: string;
    onStep: (kind: string) => void;
    completedStages?: string[];
  } = $props();

  const stepOrder = ["setup", "search", "detail", "reader", "review"];

  const steps = [
    { value: "setup", label: "1. Sito", icon: Globe },
    { value: "search", label: "2. Ricerca", icon: Search },
    { value: "detail", label: "3. Serie", icon: BookOpen },
    { value: "reader", label: "4. Pagine", icon: Images },
    { value: "review", label: "5. Verifica", icon: Check },
  ];

  let currentIdx = $derived(stepOrder.indexOf(kind));
</script>

<div class="lab-guide">
  <div class="lab-tabs progress-indicator" role="navigation" aria-label="Progresso registrazione connettore">
    {#each steps as step, index}
      {@const isCompleted = index < currentIdx || completedStages.includes(step.value)}
      {@const isCurrent = kind === step.value}
      {@const isAccessible = index <= currentIdx || isCompleted}
      <button
        type="button"
        class:chosen={isCurrent}
        class:completed={isCompleted && !isCurrent}
        disabled={!isAccessible}
        onclick={() => {
          if (isAccessible) onStep(step.value);
        }}
      >
        {#if isCompleted && !isCurrent}
          <Check size={14} class="progress-check" />
        {:else}
          <step.icon size={14} />
        {/if}
        {step.label}
      </button>
    {/each}
  </div>
</div>

<style>
  .progress-indicator {
    display: flex;
    gap: 8px;
    margin: 14px 0 16px;
    flex-wrap: wrap;
  }

  .progress-indicator button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    font-size: 13px;
    border-radius: 8px;
    transition: all 0.15s ease;
  }

  .progress-indicator button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .progress-indicator button.completed {
    border-color: rgba(34, 197, 94, 0.4);
    color: #4ade80;
  }

  :global(.progress-check) {
    color: #4ade80;
  }
</style>
