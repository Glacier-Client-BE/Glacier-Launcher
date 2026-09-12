<script lang="ts">
  /**
   * Inline download progress + cancel control - 1:1 port of
   * `Components/InlineProgress.razor` (27 lines).
   *
   * Used by the Clients, Addons, LeviLamina and Glacier panels. It had no
   * counterpart in the rewrite, so those panels had nowhere to render
   * download progress from.
   *
   * Faithful details: the progress bar and percentage only appear once
   * `progress > 0`; the cancel button renders only when an `onCancel`
   * handler is supplied (the original tests `OnCancel.HasDelegate`), and it
   * contains BOTH a spinner and the X icon; otherwise a disabled
   * spinner-only button shows while `showSpinner` is true. Percentages are
   * whole numbers, matching the original's "F0" formatting.
   */
  import IconBtn from './IconBtn.svelte'

  let {
    progress = 0,
    showSpinner = true,
    onCancel,
  }: {
    progress?: number
    showSpinner?: boolean
    onCancel?: () => void
  } = $props()

  const pct = $derived(Math.round(progress * 100))
</script>

<div class="dl-progress-row">
  {#if progress > 0}
    <div
      class="progress-bar-wrap"
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={pct}
    >
      <div class="progress-bar-fill" style="width:{pct}%"></div>
    </div>
    <span class="dl-pct">{pct}%</span>
  {/if}
  {#if onCancel}
    <IconBtn
      class="icon-btn-cancel"
      title="Cancel"
      onClick={() => onCancel?.()}
      onMouseDown={(e) => e.stopPropagation()}
    >
      {#snippet children()}
        <span class="spinner"></span>
        <i class="fa-solid fa-xmark"></i>
      {/snippet}
    </IconBtn>
  {:else if showSpinner}
    <IconBtn loading disabled title="Working" />
  {/if}
</div>
