<script lang="ts">
  import { tick } from "svelte";
  import { ChevronDown, Check } from "lucide-svelte";
  type Option = { value: string; label: string; disabled?: boolean };
  let {
    value = $bindable(""),
    options,
    label,
    onchange,
    disabled = false,
  }: {
    value?: string;
    options: Option[];
    label: string;
    onchange?: (value: string) => void;
    disabled?: boolean;
  } = $props();
  let expanded = $state(false),
    trigger = $state<HTMLButtonElement>(),
    root = $state<HTMLDivElement>();
  let position = $state({ left: 0, top: 0, width: 0, maxHeight: 280 });
  let selected = $derived(options.find((option) => option.value === value));
  const uid = `select-${crypto.randomUUID()}`;
  function close() {
    expanded = false;
  }
  async function toggle() {
    if (disabled) return;
    if (expanded) {
      close();
      return;
    }
    const rect = trigger!.getBoundingClientRect();
    const width = Math.min(Math.max(rect.width, 220), window.innerWidth - 16);
    const below = window.innerHeight - rect.bottom - 12;
    const height = Math.min(options.length * 40 + 12, 280);
    position = {
      left: Math.min(rect.left, window.innerWidth - width - 8),
      top:
        below >= height ? rect.bottom + 6 : Math.max(8, rect.top - height - 6),
      width,
      maxHeight: Math.min(height, window.innerHeight - 16),
    };
    expanded = true;
    await tick();
  }
  function choose(option: Option) {
    if (option.disabled) return;
    value = option.value;
    onchange?.(value);
    close();
    trigger?.focus();
  }
  async function key(event: KeyboardEvent, index = -1) {
    if (event.key === "Escape" && expanded) {
      event.preventDefault();
      event.stopPropagation();
      close();
      trigger?.focus();
      return;
    }
    if (event.key === "Tab") {
      close();
      return;
    }
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    event.stopPropagation();
    if (!expanded) await toggle();
    const direction = event.key === "ArrowUp" ? -1 : 1;
    let next =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? options.length - 1
          : index < 0
            ? direction === 1
              ? 0
              : options.length - 1
            : (index + direction + options.length) % options.length;
    for (
      let tries = 0;
      options[next]?.disabled && tries < options.length;
      tries++
    )
      next = (next + direction + options.length) % options.length;
    await tick();
    (
      root?.querySelector(
        `[data-option-index="${next}"]`,
      ) as HTMLButtonElement | null
    )?.focus();
  }
</script>

<svelte:window
  onpointerdown={(event) => {
    if (root && !root.contains(event.target as Node)) close();
  }}
  onresize={close}
  onscrollcapture={(event) => {
    if (expanded && root && !root.contains(event.target as Node)) close();
  }}
/>
<div class="modern-select" bind:this={root}>
  <button
    type="button"
    bind:this={trigger}
    class="select-button"
    aria-label={label}
    aria-haspopup="listbox"
    aria-expanded={expanded}
    aria-controls={expanded ? uid : undefined}
    {disabled}
    onclick={toggle}
    onkeydown={(event) => key(event)}
    ><span>{selected?.label || "Scegli…"}</span><ChevronDown
      size={15}
      class={expanded ? "rotated" : ""}
    /></button
  >
  {#if expanded}<div
      class="select-options"
      id={uid}
      role="listbox"
      aria-label={label}
      style={`left:${position.left}px;top:${position.top}px;width:${position.width}px;max-height:${position.maxHeight}px`}
    >
      {#each options as option, index}<button
          type="button"
          role="option"
          data-option-index={index}
          aria-selected={value === option.value}
          disabled={option.disabled}
          onclick={() => choose(option)}
          onkeydown={(event) => key(event, index)}
          ><span>{option.label}</span>{#if value === option.value}<Check
              size={15}
            />{/if}</button
        >{/each}
    </div>{/if}
</div>
