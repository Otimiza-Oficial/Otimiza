<script lang="ts">
  import Esqueleto from "./Esqueleto.svelte";
  // A área Tela: o Hz que cada monitor aguenta e o que está usando. A troca é em dois tempos, como o Windows faz:
  // aplica sem gravar, pergunta "Manter?" e, sem resposta no prazo, o Rust volta sozinho. A tela só conta o prazo;
  // quem decide a volta é o Rust, então o fim da contagem só relê o diagnóstico.

  import { onDestroy, onMount } from "svelte";
  import VrrDosMonitores from "./VrrDosMonitores.svelte";
  import { ponte, type DiagnosticoDaTela, type TrocaAplicada } from "../lib/ponte";

  type Aviso = { tipo: "ok" | "erro" | "atencao"; texto: string };

  let { aoMudarHz }: { aoMudarHz?: (dispositivo: string, hz: number) => void } = $props();

  let telas = $state<DiagnosticoDaTela[] | null>(null);
  let erro = $state<string | null>(null);
  let ocupado = $state<string | null>(null);
  let troca = $state<TrocaAplicada | null>(null);
  let restam = $state(0);
  let aviso = $state<Aviso | null>(null);
  let relogio: ReturnType<typeof setInterval> | undefined;

  async function carregar() {
    try {
      telas = await ponte.telas();
      for (const t of telas) if (t.atual) aoMudarHz?.(t.dispositivo, t.atual.hz);
      erro = null;
    } catch (e) {
      erro = String(e);
    }
  }

  onMount(async () => {
    await carregar();
    if (await ponte.trocaDeTelaPendente().catch(() => false)) {
      aviso = {
        tipo: "atencao",
        texto: "Uma troca de taxa estava esperando confirmação. Se você não confirmar, ela volta sozinha em segundos.",
      };
    }
  });

  onDestroy(() => clearInterval(relogio));

  function contar(t: TrocaAplicada) {
    clearInterval(relogio);
    restam = Math.ceil(t.prazo_ms / 1000);
    relogio = setInterval(() => {
      restam -= 1;
      if (restam <= 0) {
        clearInterval(relogio);
        troca = null;
        aviso = { tipo: "atencao", texto: `Sem confirmação: a tela voltou para ${t.hz_anterior} Hz.` };
        // A volta do Rust acontece no mesmo prazo: um respiro antes de reler.
        setTimeout(carregar, 1500);
      }
    }, 1000);
  }

  async function trocar(t: DiagnosticoDaTela, hz: number) {
    ocupado = t.dispositivo;
    aviso = null;
    try {
      troca = await ponte.trocarHz(t.dispositivo, hz);
      contar(troca);
    } catch (e) {
      aviso = { tipo: "erro", texto: String(e) };
    } finally {
      ocupado = null;
    }
  }

  async function manter() {
    if (!troca) return;
    const t = troca;
    clearInterval(relogio);
    troca = null;
    try {
      const fim = await ponte.confirmarTroca(t.token);
      aviso =
        fim.estado === "Mantida"
          ? { tipo: "ok", texto: `Pronto: ${fim.hz} Hz. Para voltar, use o Histórico.` }
          : { tipo: "atencao", texto: `A tela voltou para ${fim.hz} Hz.` };
    } catch (e) {
      aviso = { tipo: "erro", texto: String(e) };
    } finally {
      await carregar();
    }
  }

  async function voltar() {
    if (!troca) return;
    const t = troca;
    clearInterval(relogio);
    troca = null;
    try {
      const fim = await ponte.voltarTroca(t.token);
      aviso = { tipo: "ok", texto: `Voltou para ${fim.hz} Hz.` };
    } catch (e) {
      aviso = { tipo: "erro", texto: String(e) };
    } finally {
      await carregar();
    }
  }

  function resolucao(t: DiagnosticoDaTela) {
    return t.atual ? `${t.atual.largura}×${t.atual.altura}` : "resolução não lida";
  }
</script>

<div class="area-tela">
  {#if troca}
    <div class="manter" role="alertdialog" aria-labelledby="manter-titulo">
      <div>
        <strong id="manter-titulo">A imagem está boa em {troca.hz_novo} Hz?</strong>
        <span class="apoio">Sem resposta, volta para {troca.hz_anterior} Hz em {restam} s.</span>
      </div>
      <button class="btn" onclick={voltar}>Voltar</button>
      <button class="btn primario" onclick={manter}>Manter</button>
    </div>
  {/if}

  {#if aviso}
    <div class="recado" role="status">
      <span class="ponto" data-tom={aviso.tipo === "ok" ? "ok" : aviso.tipo === "erro" ? "perigo" : "atencao"}></span>
      <div><span>{aviso.texto}</span></div>
    </div>
  {/if}

  {#if erro}
    <div class="recado" role="alert">
      <span class="ponto" data-tom="perigo"></span>
      <div>
        <strong>Não consegui ler os monitores.</strong>
        <span class="apoio">{erro}</span>
        <button class="btn" onclick={carregar}>Tentar de novo</button>
      </div>
    </div>
  {:else if telas === null}
    <Esqueleto rotulo="Lendo os monitores" />
  {:else if telas.length === 0}
    <p class="vazio">Nenhum monitor ativo foi encontrado.</p>
  {:else}
    {#each telas as t (t.dispositivo)}
      <section class="secao monitor">
        <div class="cabeca">
          <div>
            <h2>{t.nome || t.dispositivo}</h2>
            <span class="apoio">{resolucao(t)}{t.principal ? " · principal" : ""}</span>
          </div>
          <div class="hz">
            <span class="numero">{t.atual ? t.atual.hz : "—"}</span>
            <span class="apoio">Hz agora</span>
          </div>
        </div>
        <p class="texto">
          <span class="ponto" data-tom={t.veredito.estado === "NoMaximo" ? "ok" : t.veredito.estado === "ModosIlegiveis" ? "neutro" : "atencao"}></span>
          {t.texto}
        </p>
        {#if t.veredito.estado === "UmClique"}
          <div>
            <button
              class="btn primario"
              disabled={ocupado !== null || troca !== null}
              onclick={() => t.veredito.estado === "UmClique" && trocar(t, t.veredito.hz_alvo)}
            >
              {ocupado === t.dispositivo ? "Trocando…" : `Usar ${t.veredito.hz_alvo} Hz`}
            </button>
          </div>
        {/if}
      </section>
    {/each}
  {/if}
  <VrrDosMonitores />
</div>

<style>
  .area-tela {
    display: grid;
    gap: 12px;
    padding-top: 12px;
  }
  .vazio {
    padding: 24px 0;
    color: var(--tinta-2);
  }
  .manter {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 12px 12px 16px;
    border-radius: var(--raio-2);
    background: var(--acao-suave);
  }
  .manter > div {
    flex: 1;
    display: grid;
  }
  .manter strong {
    font-weight: 500;
  }
  .monitor:first-of-type {
    border-top: 0;
  }
  .cabeca {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 12px;
  }
  .hz {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }
  .hz .numero {
    font-size: var(--t-numero);
    font-weight: 600;
    line-height: 1;
  }
  .texto {
    display: flex;
    gap: 10px;
    align-items: baseline;
    color: var(--tinta-2);
    max-width: 62ch;
  }
  .texto .ponto {
    transform: translateY(-1px);
  }
</style>
