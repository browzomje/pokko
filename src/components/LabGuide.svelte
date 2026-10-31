<script lang="ts">
  import { Search, BookOpen, Images, Globe, Check } from "lucide-svelte";
  import { t } from "../lib/i18n";

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

  let steps = $derived([
    { value: "setup", label: t("guide.step1"), icon: Globe },
    { value: "search", label: t("guide.step2"), icon: Search },
    { value: "detail", label: t("guide.step3"), icon: BookOpen },
    { value: "reader", label: t("guide.step4"), icon: Images },
    { value: "review", label: t("guide.step5"), icon: Check },
  ]);

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
    margin: 0;
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
