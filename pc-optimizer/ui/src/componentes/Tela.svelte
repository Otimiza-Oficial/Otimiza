<script lang="ts">
  // A área Tela: o Hz que cada monitor aguenta e o que está usando. A troca é em dois tempos, como o Windows faz:
  // aplica sem gravar, pergunta "Manter?" e, sem resposta no prazo, o Rust volta sozinho. A tela só conta o prazo;
  // quem decide a volta é o Rust, então o fim da contagem só relê o diagnóstico.

  import { onDestroy, onMount } from "svelte";
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
    <div class="aviso manter" data-tipo="atencao" role="alertdialog" aria-labelledby="manter-titulo">
      <strong id="manter-titulo">A imagem está boa em {troca.hz_novo} Hz?</strong>
      <span>Sem resposta, volta para {troca.hz_anterior} Hz em {restam} s.</span>
      <div class="botoes">
        <button class="principal" onclick={manter}>Manter</button>
        <button class="secundario" onclick={voltar}>Voltar</button>
      </div>
    </div>
  {/if}

  {#if aviso}
    <div class="aviso" data-tipo={aviso.tipo} role="status"><span>{aviso.texto}</span></div>
  {/if}

  {#if erro}
    <div class="aviso" data-tipo="erro" role="alert">
      <strong>Não consegui ler os monitores.</strong>
      <span>{erro}</span>
      <button class="secundario" onclick={carregar}>Tentar de novo</button>
    </div>
  {:else if telas === null}
    <p class="vazio">Lendo os monitores…</p>
  {:else if telas.length === 0}
    <p class="vazio">Nenhum monitor ativo foi encontrado.</p>
  {:else}
    <ul class="monitores">
      {#each telas as t (t.dispositivo)}
        <li>
          <div class="cabeca">
            <div class="titulo">
              <strong>{t.nome || t.dispositivo}</strong>
              <span>{resolucao(t)}{t.principal ? " · principal" : ""}</span>
            </div>
            <div class="hz">
              <span class="numero">{t.atual ? t.atual.hz : "—"}</span><small>Hz agora</small>
            </div>
          </div>
          <p class="texto">{t.texto}</p>
          {#if t.veredito.estado === "UmClique"}
            <button
              class="principal"
              disabled={ocupado !== null || troca !== null}
              onclick={() => t.veredito.estado === "UmClique" && trocar(t, t.veredito.hz_alvo)}
            >
              {ocupado === t.dispositivo ? "Trocando…" : `Usar ${t.veredito.hz_alvo} Hz`}
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .area-tela {
    display: grid;
    gap: 12px;
    margin-top: 20px;
  }
  .aviso {
    display: grid;
    gap: 4px;
    justify-items: start;
    padding: 12px 16px 12px 18px;
    border-radius: 16px;
    background: var(--vidro);
    border-left: 4px solid var(--sucesso);
    font-size: 14px;
  }
  .aviso[data-tipo="atencao"] {
    border-left-color: var(--atencao);
  }
  .aviso[data-tipo="erro"] {
    border-left-color: var(--perigo);
  }
  .aviso span {
    color: var(--tinta-suave);
  }
  .manter {
    background: var(--vidro-denso);
    box-shadow: var(--vidro-sombra);
  }
  .botoes {
    display: flex;
    gap: 8px;
    margin-top: 6px;
  }
  .vazio {
    margin: 8px 0 0;
    color: var(--tinta-suave);
  }
  button {
    font: inherit;
    cursor: pointer;
  }
  button:disabled {
    cursor: default;
    opacity: 0.55;
  }
  .principal {
    justify-self: start;
    padding: 8px 18px;
    border: 0;
    border-radius: var(--raio-botao);
    font-weight: 600;
    color: white;
    background: var(--ceu-700);
  }
  .secundario {
    padding: 8px 16px;
    border-radius: var(--raio-botao);
    border: 1px solid var(--vidro-borda);
    background: var(--vidro);
    color: var(--tinta);
  }
  .monitores {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  .monitores > li {
    display: grid;
    gap: 8px;
    padding: 14px 16px;
    border-radius: 16px;
    background: var(--vidro);
    border: 1px solid var(--vidro-borda);
  }
  .cabeca {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .titulo {
    flex: 1;
    display: grid;
  }
  .titulo span {
    color: var(--tinta-suave);
    font-size: 12px;
  }
  .hz {
    display: grid;
    justify-items: end;
  }
  .hz .numero {
    font-size: 28px;
    font-weight: 700;
    line-height: 1;
  }
  .hz small {
    color: var(--tinta-suave);
    font-size: 11px;
  }
  .texto {
    margin: 0;
    font-size: 14px;
    color: var(--tinta-suave);
  }
</style>
