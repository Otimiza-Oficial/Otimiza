<script lang="ts">
  import Esqueleto from "./Esqueleto.svelte";
  // A área Histórico: tudo o que o Otimiza mudou neste PC, com o valor de antes como foi GRAVADO, e o desfazer um por
  // um ou tudo de uma vez. Lista vazia com histórico ilegível não é "nada mudou": a leitura aparece antes da lista.
  // Onde o Rust manda `null` (item fora do catálogo), a tela não afirma nem nega admin ou reinício.

  import { onMount } from "svelte";
  import SaudeDoHistorico from "./SaudeDoHistorico.svelte";
  import QuandoPiorou from "./QuandoPiorou.svelte";
  import { ponte, type Historico, type Mudanca, type Parte, type RelatorioDeDesfazer } from "../lib/ponte";

  type Aviso = { tipo: "ok" | "erro" | "atencao"; texto: string };

  const NOME_DA_PARTE: Record<Parte, string> = {
    Automaticos: "modos automáticos",
    MotorDeEnergia: "motor de energia",
    NucleosPorJogo: "regras de núcleos por jogo",
    Historico: "histórico de mudanças",
    PlanoOtimiza: "plano de energia OTIMIZA",
  };

  let historico = $state<Historico | null>(null);
  let erroDeLeitura = $state<string | null>(null);
  let admin = $state<boolean | null>(null);
  /** O id que está sendo desfeito agora, ou "tudo". Um de cada vez: o histórico é um arquivo só. */
  let ocupado = $state<string | null>(null);
  let confirmandoTudo = $state(false);
  let aviso = $state<Aviso | null>(null);
  let abertos = $state<Record<string, boolean>>({});

  const mudancas = $derived(historico?.mudancas ?? []);
  const desfaziveis = $derived(mudancas.filter((m) => m.desfazivel).length);
  const algumPedeAdmin = $derived(mudancas.some((m) => m.desfazivel && m.precisa_de_admin === true));

  async function carregar() {
    try {
      historico = await ponte.historico();
      erroDeLeitura = null;
    } catch (e) {
      erroDeLeitura = String(e);
    }
  }

  onMount(() => {
    void carregar();
    ponte.administrador().then((a) => (admin = a)).catch(() => (admin = null));
  });

  function quando(segundos: number) {
    const data = new Date(segundos * 1000);
    const passou = (Date.now() - data.getTime()) / 1000;
    const hora = data.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
    if (passou < 60) return "agora há pouco";
    if (passou < 3600) return `há ${Math.floor(passou / 60)} min`;
    const hoje = new Date();
    const ontem = new Date(hoje.getFullYear(), hoje.getMonth(), hoje.getDate() - 1);
    if (data.toDateString() === hoje.toDateString()) return `hoje, ${hora}`;
    if (data.toDateString() === ontem.toDateString()) return `ontem, ${hora}`;
    return `${data.toLocaleDateString("pt-BR", { day: "numeric", month: "short" })}, ${hora}`;
  }

  /** Sem administrador, item que pede administrador não tem como voltar: o botão diz isso em vez de falhar. */
  function bloqueado(m: Mudanca) {
    return m.precisa_de_admin === true && admin === false;
  }

  async function desfazer(m: Mudanca) {
    ocupado = m.id;
    aviso = null;
    try {
      const r = await ponte.desfazer(m.id);
      const reiniciar = r.success && (r.requires_restart || r.requires_logoff);
      aviso = r.success
        ? {
            tipo: reiniciar ? "atencao" : "ok",
            texto: `${m.nome}: voltou ao que era.${r.requires_restart ? " Reinicie o PC para valer." : r.requires_logoff ? " Saia e entre de novo no Windows para valer." : ""}`,
          }
        : { tipo: "erro", texto: `${m.nome}: ${r.message}` };
    } catch (e) {
      aviso = { tipo: "erro", texto: `${m.nome}: ${String(e)}` };
    } finally {
      ocupado = null;
      await carregar();
    }
  }

  function resumo(r: RelatorioDeDesfazer): Aviso {
    const voltaram = r.partes.filter((p) => p.desfecho.estado === "Voltou");
    const falhas = r.partes.flatMap((p) =>
      p.desfecho.estado === "Falhou" ? [`${NOME_DA_PARTE[p.parte]}: ${p.desfecho.motivo}`] : []
    );
    const itensQueFalharam = r.itens.filter((i) => !i.success).map((i) => `${i.name}: ${i.message}`);
    if (voltaram.length === 0 && falhas.length === 0 && itensQueFalharam.length === 0) {
      return { tipo: "ok", texto: "Nada a desfazer: o Otimiza não tem nenhuma mudança ativa neste PC." };
    }
    if (r.tudo_voltou) {
      const reiniciar = r.itens.some((i) => i.success && i.requires_restart);
      return {
        tipo: reiniciar ? "atencao" : "ok",
        texto: `Tudo desfeito: ${voltaram.map((p) => NOME_DA_PARTE[p.parte]).join(", ")}.${reiniciar ? " Reinicie o PC para tudo valer." : ""}`,
      };
    }
    return { tipo: "erro", texto: `Desfeito só em parte. ${[...falhas, ...itensQueFalharam].join(" · ")}` };
  }

  async function desfazerTudo() {
    if (!confirmandoTudo) {
      confirmandoTudo = true;
      return;
    }
    confirmandoTudo = false;
    ocupado = "tudo";
    aviso = null;
    try {
      aviso = resumo(await ponte.desfazerTudo());
    } catch (e) {
      aviso = { tipo: "erro", texto: String(e) };
    } finally {
      ocupado = null;
      await carregar();
    }
  }

  async function abrirComoAdministrador() {
    try {
      const msg = await ponte.abrirComoAdministrador();
      if (msg) aviso = { tipo: "atencao", texto: msg };
    } catch (e) {
      aviso = { tipo: "erro", texto: String(e) };
    }
  }
</script>

<div class="historico">
  <SaudeDoHistorico />
  <QuandoPiorou />
  {#if historico?.leitura.estado === "Ilegivel"}
    <div class="recado" role="alert">
      <span class="ponto" data-tom="perigo"></span>
      <div>
        <strong>Não consegui ler o histórico de mudanças.</strong>
        <span class="apoio">
          A lista pode estar incompleta: o que o Otimiza mudou antes pode continuar no PC. {historico.leitura.motivo}
        </span>
        {#if historico.leitura.guardado_em}
          <span class="apoio">O arquivo original foi guardado em <code>{historico.leitura.guardado_em}</code>.</span>
        {/if}
      </div>
    </div>
  {/if}

  {#if erroDeLeitura}
    <div class="recado" role="alert">
      <span class="ponto" data-tom="perigo"></span>
      <div>
        <strong>O histórico não respondeu.</strong>
        <span class="apoio">{erroDeLeitura}</span>
        <button class="btn" onclick={carregar}>Tentar de novo</button>
      </div>
    </div>
  {/if}

  {#if algumPedeAdmin && admin === false}
    <div class="recado">
      <span class="ponto" data-tom="atencao"></span>
      <div>
        <span>Parte disto só volta com o Otimiza aberto como administrador.</span>
        <button class="btn" onclick={abrirComoAdministrador}>Abrir como administrador</button>
      </div>
    </div>
  {/if}

  {#if aviso}
    <div class="recado" role="status">
      <span class="ponto" data-tom={aviso.tipo === "ok" ? "ok" : aviso.tipo === "erro" ? "perigo" : "atencao"}></span>
      <div><span>{aviso.texto}</span></div>
    </div>
  {/if}

  {#if historico === null && !erroDeLeitura}
    <Esqueleto linhas={4} rotulo="Lendo o histórico" />
  {:else if historico && mudancas.length === 0 && historico.leitura.estado === "Ok"}
    <p class="vazio">O Otimiza não tem nenhuma mudança ativa neste PC. O que você aplicar (o "Otimizar agora", na Visão geral, ou um ajuste do Sistema) aparece aqui, cada um com o valor de antes e o desfazer.</p>
  {:else if mudancas.length > 0}
    <div class="topo">
      <p class="apoio">
        {mudancas.length}
        {mudancas.length === 1 ? "mudança ativa" : "mudanças ativas"}, da mais nova à mais antiga
      </p>
      {#if desfaziveis > 0}
        <button
          class="btn"
          class:perigo={confirmandoTudo}
          disabled={ocupado !== null}
          onclick={desfazerTudo}
          onblur={() => (confirmandoTudo = false)}
        >
          {#if ocupado === "tudo"}
            Desfazendo tudo…
          {:else if confirmandoTudo}
            Confirmar: desfazer {desfaziveis}
          {:else}
            Desfazer tudo
          {/if}
        </button>
      {/if}
    </div>

    <ol class="lista">
      {#each mudancas as m (m.id)}
        <li>
          <div class="item">
            <div class="cabeca">
              <strong>{m.nome}</strong>
              <span class="apoio">
                {quando(m.quando)}{m.precisa_de_admin === true ? " · administrador" : ""}{m.precisa_reiniciar === true
                  ? " · vale depois de reiniciar"
                  : ""}
              </span>
            </div>
            {#if m.o_que_mudou.length > 0}
              <button class="btn quieto" aria-expanded={abertos[m.id] === true} onclick={() => (abertos[m.id] = !abertos[m.id])}>
                {abertos[m.id] ? "Esconder" : "Ver"} o que mudou
              </button>
              {#if abertos[m.id]}
                <ul class="valores">
                  {#each m.o_que_mudou as linha, i (i)}
                    <li><code>{linha}</code></li>
                  {/each}
                </ul>
              {/if}
            {/if}
          </div>
          {#if m.desfazivel}
            <button
              class="btn"
              disabled={ocupado !== null || bloqueado(m)}
              title={bloqueado(m) ? "Precisa do Otimiza aberto como administrador" : undefined}
              onclick={() => desfazer(m)}
            >
              {ocupado === m.id ? "Desfazendo…" : "Desfazer"}
            </button>
          {:else}
            <span class="apoio sem-volta" title="Apagou arquivos de verdade: não há o que devolver">Não tem volta</span>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
</div>

<style>
  .historico {
    display: grid;
    gap: 12px;
    padding-top: 12px;
  }
  .vazio {
    padding: 24px 0;
    color: var(--tinta-2);
  }
  .topo {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .lista > li {
    align-items: flex-start;
  }
  .item {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 4px;
    justify-items: start;
  }
  .cabeca {
    display: grid;
    gap: 2px;
  }
  .cabeca strong {
    font-weight: 500;
  }
  .sem-volta {
    padding-top: 8px;
  }
  .valores {
    margin: 4px 0 0;
    padding: 8px 12px;
    list-style: none;
    display: grid;
    gap: 4px;
    width: 100%;
    border-radius: var(--raio-1);
    background: var(--superficie-2);
  }
  code {
    font-family: "Cascadia Mono", Consolas, monospace;
    font-size: var(--t-apoio);
    color: var(--tinta-2);
    word-break: break-all;
    user-select: text;
  }
</style>
