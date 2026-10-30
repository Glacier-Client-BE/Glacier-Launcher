<script lang="ts">
  import { t } from './i18nStore.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import PanelHeader from './components/PanelHeader.svelte'

  let { edition, onBack, onNavigate }: {
    edition: 'bedrock' | 'java'
    onBack?: () => void
    onNavigate?: (view: string) => void
  } = $props()

  /** Port of `OpenUrl` (Home.razor.cs:670) — routed through the `open_url`
   * command since there's no browser chrome around the WebView2 host for
   * `window.open` to hand off to. */
  function openUrl(url: string) {
    if (url) invoke('open_url', { url }).catch(() => {})
  }

  type Credit = {
    /** Image path; empty when `faIcon` (a Font Awesome class) is used instead. */
    icon: string
    faIcon?: string
    name: string
    /** i18n key for the subtitle line. */
    sub: string
    /** Each link's tooltip is an i18n key. */
    links: { tooltip: string; icon: string; url: string }[]
  }

  const clients: Credit[] = [
    {
      icon: 'images/clients/latite.png', name: 'Latite Client', sub: 'credits.latiteSub',
      links: [
        { tooltip: 'tt.githubReleases', icon: 'fa-brands fa-github', url: 'https://github.com/Imrglop/Latite-Releases' },
        { tooltip: 'tt.discord', icon: 'fa-brands fa-discord', url: 'https://discord.gg/latite' },
      ],
    },
    {
      icon: 'images/clients/flarial.svg', name: 'Flarial Client', sub: 'credits.flarialSub',
      links: [
        { tooltip: 'tt.website', icon: 'fa-solid fa-globe', url: 'https://flarial.xyz' },
        { tooltip: 'tt.discord', icon: 'fa-brands fa-discord', url: 'https://discord.gg/flarial' },
      ],
    },
    {
      icon: 'images/clients/oderso.png', name: 'OderSo Client', sub: 'credits.oderSoSub',
      links: [
        { tooltip: 'tt.github', icon: 'fa-brands fa-github', url: 'https://github.com/MasonOderSo/oderso-data' },
      ],
    },
    {
      icon: '', faIcon: 'fa-solid fa-sun', name: 'Horizon', sub: 'credits.horizonSub',
      links: [
        { tooltip: 'tt.website', icon: 'fa-solid fa-globe', url: 'https://playhorizon.dev' },
        { tooltip: 'tt.github', icon: 'fa-brands fa-github', url: 'https://github.com/horizon-rust/horizon' },
      ],
    },
  ]

  const tweaks: Credit[] = [
    {
      icon: 'images/vanilla-tweaks.png', name: 'VanillaTweaks', sub: 'credits.vanillaTweaksSub',
      links: [
        { tooltip: 'tt.website', icon: 'fa-solid fa-globe', url: 'https://vanillatweaks.net' },
      ],
    },
    {
      icon: 'images/bedrock-tweaks.png', name: 'BedrockTweaks', sub: 'credits.bedrockTweaksSub',
      links: [
        { tooltip: 'tt.website', icon: 'fa-solid fa-globe', url: 'https://bedrocktweaks.net' },
        { tooltip: 'tt.github', icon: 'fa-brands fa-github', url: 'https://github.com/BedrockTweaks/Files' },
      ],
    },
  ]
</script>

<PanelHeader title={t('panel.credits')} {onBack} />

<div class="panel-body">
  <div class="credits-hero">
    <div class="credits-hero-icon">
      <img src="images/icon.png" alt="" />
    </div>
    <div class="credits-hero-meta">
      <span class="credits-hero-name">Glacier Launcher</span>
      <span class="credits-hero-sub">{t('credits.builtBy')}</span>
    </div>
    <div class="credits-card-actions">
      <button class="credits-link-btn" data-tooltip={t('tt.github')} aria-label={t('tt.github')} onclick={() => openUrl('https://github.com/Glacier-Client-BE')}>
        <i class="fa-brands fa-github"></i>
      </button>
      <button class="credits-link-btn" data-tooltip={t('tt.website')} aria-label={t('tt.website')} onclick={() => openUrl('https://glacierclient.xyz')}>
        <i class="fa-solid fa-globe"></i>
      </button>
      <button class="credits-link-btn" data-tooltip={t('tt.discord')} aria-label={t('tt.discord')} onclick={() => openUrl('https://discord.glacierclient.xyz')}>
        <i class="fa-brands fa-discord"></i>
      </button>
    </div>
  </div>

  <span class="panel-section-label"><i class="fa-solid fa-layer-group"></i> {t('label.clients')}</span>

  <div class="credits-grid">
    {#each clients as c (c.name)}
      <div class="credits-card">
        <div class="credits-card-icon credits-icon-pad">
          {#if c.icon}
            <img src={c.icon} class="credits-icon-img" alt="" />
          {:else}
            <i class={c.faIcon}></i>
          {/if}
        </div>
        <div class="credits-card-meta">
          <span class="credits-card-name">{c.name}</span>
          <span class="credits-card-sub">{t(c.sub)}</span>
        </div>
        <div class="credits-card-actions">
          {#each c.links as l (l.tooltip)}
            <button class="credits-link-btn" data-tooltip={t(l.tooltip)} aria-label={t(l.tooltip)} onclick={() => openUrl(l.url)}>
              <i class={l.icon}></i>
            </button>
          {/each}
        </div>
      </div>
    {/each}
  </div>

  <span class="panel-section-label"><i class="fa-solid fa-wand-magic-sparkles"></i> {t('label.tweaks')}</span>

  <div class="credits-grid">
    {#each tweaks as tw (tw.name)}
      <div class="credits-card">
        <div class="credits-card-icon credits-icon-pad"><img src={tw.icon} class="credits-icon-img" alt="" /></div>
        <div class="credits-card-meta">
          <span class="credits-card-name">{tw.name}</span>
          <span class="credits-card-sub">{t(tw.sub)}</span>
        </div>
        <div class="credits-card-actions">
          {#each tw.links as l (l.tooltip)}
            <button class="credits-link-btn" data-tooltip={t(l.tooltip)} aria-label={t(l.tooltip)} onclick={() => openUrl(l.url)}>
              <i class={l.icon}></i>
            </button>
          {/each}
        </div>
      </div>
    {/each}
  </div>

  <div class="credits-oss-row">
    <i class="fa-solid fa-code-branch credits-oss-icon"></i>
    <span>{t('credits.openSource')}</span>
    <button class="btn-sm" onclick={() => openUrl('https://github.com/Glacier-Client-BE/Glacier-Launcher')}>
      <i class="fa-brands fa-github"></i> {t('common.view')}
    </button>
  </div>
</div>

<style>
  .credits-icon-pad {
    padding: 6px;
  }
  .credits-icon-img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .credits-oss-icon {
    color: var(--accent-hover);
  }
</style>
