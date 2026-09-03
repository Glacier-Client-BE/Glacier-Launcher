<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    open,
    title = '',
    icon = '',
    iconColor,
    class: cls = '',
    closeOnBackdrop = true,
    onClose,
    children,
  }: {
    open: boolean
    title?: string
    icon?: string
    iconColor?: string
    class?: string
    closeOnBackdrop?: boolean
    onClose?: () => void
    children?: Snippet
  } = $props()

  function handleBackdropClick() {
    if (closeOnBackdrop) onClose?.()
  }
</script>

{#if open}
  <div class="modal-overlay" role="presentation" onclick={handleBackdropClick}>
    <div
      class="modal-box {cls}"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="modal-header">
        <span class="modal-title">
          {#if icon}
            <i class={icon} style:color={iconColor ?? 'var(--accent)'} style:margin-right="8px"></i>
          {/if}
          {title}
        </span>
        <button type="button" class="modal-close-btn" aria-label="Close" onclick={() => onClose?.()}>
          <i class="fa-solid fa-xmark"></i>
        </button>
      </div>
      <div class="modal-body">
        {#if children}{@render children()}{/if}
      </div>
    </div>
  </div>
{/if}

<!-- .modal-overlay / .modal-box / .modal-header / .modal-title /
     .modal-close-btn / .modal-body are defined in app.css (ported 1:1) —
     no scoped styles here. -->
