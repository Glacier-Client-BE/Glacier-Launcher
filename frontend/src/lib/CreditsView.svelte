<script lang="ts">
  // Ported 1:1 from the "credits" panel in Home.razor (lines 5495-5646).
  // Static content — names, roles and links copied verbatim from the source,
  // nothing fabricated or updated.
  import { invoke } from '@tauri-apps/api/core'
  import EditionTabs from './components/EditionTabs.svelte'
  import PanelHeader from './components/PanelHeader.svelte'

  let { edition, onBack, onNavigate }: {
    edition: 'bedrock' | 'java'
    onBack?: () => void
    onNavigate?: (view: string) => void
  } = $props()

  /**
   * Port of `OpenUrl` (Home.razor.cs:670). Goes through the `open_url` command
   * rather than `window.open`: there is no browser chrome around the WebView2
   * host, so `window.open` left every link on this panel dead.
   */
  function openUrl(url: string) {
    if (url) invoke('open_url', { url }).catch(() => {})
  }
</script>

<PanelHeader title="Credits" {onBack} />

<div class="panel-body">
  <span class="panel-section-label">Launcher</span>

  <div class="credits-card credits-card-glacier">
    <div class="credits-card-icon" style="padding:6px;">
      <img
        src="https://media.discordapp.net/attachments/1252981510223429693/1491099168612028497/1000167923-removebg-preview.png?ex=69d675d1&is=69d52451&hm=f8489d83a088d107f933b54625e9ebd93dd1d726788804f0dbc498720a3b689f&=&width=320&height=320"
        style="width:100%;height:100%;object-fit:contain;" alt="" />
    </div>
    <div class="credits-card-meta">
      <span class="credits-card-name">Glacier Launcher</span>
      <span class="credits-card-sub">Built by Pepe · Glacier Productions</span>
    </div>
    <div class="credits-card-actions">
      <button class="credits-link-btn" data-tooltip="GitHub" aria-label="GitHub" onclick={() => openUrl('https://github.com/Glacier-Client-BE')}>
        <i class="fa-brands fa-github"></i>
      </button>
      <button class="credits-link-btn" data-tooltip="Website" aria-label="Website" onclick={() => openUrl('https://glacierclient.xyz')}>
        <i class="fa-solid fa-globe"></i>
      </button>
      <button class="credits-link-btn" data-tooltip="Discord" aria-label="Discord" onclick={() => openUrl('https://discord.glacierclient.xyz')}>
        <i class="fa-brands fa-discord"></i>
      </button>
    </div>
  </div>

  <span class="panel-section-label">Clients</span>

  <div class="credits-card">
    <div class="credits-card-icon" style="padding:6px;"><img src="images/clients/latite.png" style="width:100%;height:100%;object-fit:contain;" alt="" /></div>
    <div class="credits-card-meta">
      <span class="credits-card-name">Latite Client</span>
      <span class="credits-card-sub">by Imrglop &amp; contributors</span>
    </div>
    <div class="credits-card-actions">
      <button class="credits-link-btn" data-tooltip="GitHub Releases" aria-label="GitHub Releases" onclick={() => openUrl('https://github.com/Imrglop/Latite-Releases')}>
        <i class="fa-brands fa-github"></i>
      </button>
      <button class="credits-link-btn" data-tooltip="Discord" aria-label="Discord" onclick={() => openUrl('https://discord.gg/latite')}>
        <i class="fa-brands fa-discord"></i>
      </button>
    </div>
  </div>

  <div class="credits-card">
    <div class="credits-card-icon" style="padding:6px;"><img src="images/clients/flarial.svg" style="width:100%;height:100%;object-fit:contain;" alt="" /></div>
    <div class="credits-card-meta">
      <span class="credits-card-name">Flarial Client</span>
      <span class="credits-card-sub">by the Flarial team</span>
    </div>
    <div class="credits-card-actions">
      <button class="credits-link-btn" data-tooltip="Website" aria-label="Website" onclick={() => openUrl('https://flarial.xyz')}>
        <i class="fa-solid fa-globe"></i>
      </button>
      <button class="credits-link-btn" data-tooltip="Discord" aria-label="Discord" onclick={() => openUrl('https://discord.gg/flarial')}>
        <i class="fa-brands fa-discord"></i>
      </button>
    </div>
  </div>

  <div class="credits-card">
    <div class="credits-card-icon" style="padding:6px;"><img src="images/clients/oderso.png" style="width:100%;height:100%;object-fit:contain;" alt="" /></div>
    <div class="credits-card-meta">
      <span class="credits-card-name">OderSo Client</span>
      <span class="credits-card-sub">by MasonOderSo</span>
    </div>
    <div class="credits-card-actions">
      <button class="credits-link-btn" data-tooltip="GitHub" aria-label="GitHub" onclick={() => openUrl('https://github.com/MasonOderSo/oderso-data')}>
        <i class="fa-brands fa-github"></i>
      </button>
    </div>
  </div>

  <span class="panel-section-label">Tweaks</span>

  <div class="credits-card">
    <div class="credits-card-icon" style="padding:6px;"><i class="fa-solid fa-wand-magic-sparkles" style="color:var(--accent);font-size:20px;"></i></div>
    <div class="credits-card-meta">
      <span class="credits-card-name">VanillaTweaks</span>
      <span class="credits-card-sub">Java resource packs — by the VanillaTweaks team</span>
    </div>
    <div class="credits-card-actions">
      <button class="credits-link-btn" data-tooltip="Website" aria-label="Website" onclick={() => openUrl('https://vanillatweaks.net')}>
        <i class="fa-solid fa-globe"></i>
      </button>
    </div>
  </div>

  <div class="credits-card">
    <div class="credits-card-icon" style="padding:6px;"><i class="fa-solid fa-wand-magic-sparkles" style="color:var(--accent);font-size:20px;"></i></div>
    <div class="credits-card-meta">
      <span class="credits-card-name">BedrockTweaks</span>
      <span class="credits-card-sub">Bedrock resource packs — by the BedrockTweaks team</span>
    </div>
    <div class="credits-card-actions">
      <button class="credits-link-btn" data-tooltip="Website" aria-label="Website" onclick={() => openUrl('https://bedrocktweaks.net')}>
        <i class="fa-solid fa-globe"></i>
      </button>
      <button class="credits-link-btn" data-tooltip="GitHub" aria-label="GitHub" onclick={() => openUrl('https://github.com/BedrockTweaks/Files')}>
        <i class="fa-brands fa-github"></i>
      </button>
    </div>
  </div>

  <span class="panel-section-label">Open Source</span>

  <div class="credits-oss-row">
    <i class="fa-solid fa-code-branch" style="color:var(--accent);"></i>
    <span>Glacier Launcher is open source. Contributions and forks are welcome.</span>
    <button class="btn-sm" onclick={() => openUrl('https://github.com/Glacier-Client-BE/Glacier-Launcher')}>
      <i class="fa-brands fa-github"></i> View
    </button>
  </div>
</div>

<EditionTabs active="credits" {edition} onNavigate={(v) => onNavigate?.(v)} />
