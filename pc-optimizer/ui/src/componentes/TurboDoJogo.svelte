<script lang="ts">
  // O Turbo neste jogo (página do jogo, aba Jogos): a receita que o Turbo guardou, o que ela liga, se a prova A-B
  // confirmou, e a escolha "Menor atraso" × "Mais FPS" deste jogo (jogo competitivo já vem em menor atraso). Ligar e
  // medir continuam no Turbo, com o jogo aberto.
  import { ponte, type TurboDoJogoNaPagina } from "../lib/ponte";

  let { jogo }: { jogo: string } = $props();

  let t = $state<TurboDoJogoNaPagina | null>(null);
  let ocupado = $state(false);
  let recado = $state<string | null>(null);

  $effect(() => {
    void ponte.turboDoJogo(jogo, null).then((x) => (t = x)).catch(() => (t = null));
  });

  async function escolher(menor: boolean) {
    if (!t || t.menor_atraso === menor) return;
    ocupado = true;
    const tinha = t.receita != null;
    try {
      t = await ponte.turboDoJogo(jogo, menor);
      recado = tinha && !t.receita ? "A receita era da outra escolha: o próximo Turbo mede este jogo de novo." : null;
    } catch (e) {
      recado = String(e);
    } finally {
      ocupado = false;
    }
  }

  const GARGALO: Record<string, string> = {
    Placa: "a placa de vídeo",
    Processador: "o processador",
    Memoria: "a memória",
  };
  const PROVA: Record<string, { texto: string; tom: "ok" | "atencao" | "perigo" }> = {
    Ganhou: { texto: "Provado: ganhou", tom: "ok" },
    Piorou: { texto: "Reprovado: não liga mais", tom: "perigo" },
    Indistinguivel: { texto: "Prova sem diferença", tom: "atencao" },
    SemComparacaoJusta: { texto: "Prova sem comparação justa", tom: "atencao" },
  };

  const pecas = $derived.by(() => {
    const x = t?.receita?.perfil.receita;
    if (!x) return [];
    const l: string[] = [];
    if (x.escala) l.push(`Escala ${x.escala}`);
    if (x.multiplicador != null && x.multiplicador !== 1) l.push(x.multiplicador === 0 ? "Frame Boost Auto" : `Frame Boost ${x.multiplicador}×`);
    if (x.energia) l.push("Energia de jogo");
    if (x.memoria) l.push("Memória livre para o jogo");
    return l;
  });

  const data = (s: number) => new Date(s * 1000).toLocaleDateString("pt-BR", { day: "2-digit", month: "2-digit" });
</script>

{#if t && t.id}
  {@const r = t.receita}
  {@const prova = r?.perfil.prova ? PROVA[r.perfil.prova] : null}
  <div class="turbo-do-jogo">
    <div class="cabeca">
      <strong>Turbo neste jogo</strong>
      {#if r}
        {#if prova}<span class="selo" data-tom={prova.tom}>{prova.texto}</span>{:else}<span class="selo estimado">ainda sem prova</span>{/if}
      {/if}
    </div>

    <div class="escolha">
      <div class="segmento" role="radiogroup" aria-label="Atraso neste jogo">
        <button role="radio" aria-checked={t.menor_atraso} disabled={ocupado} onclick={() => escolher(true)}>Menor atraso</button>
        <button role="radio" aria-checked={!t.menor_atraso} disabled={ocupado} onclick={() => escolher(false)}>Mais FPS</button>
      </div>
      <span class="apoio pequeno">
        {t.menor_atraso
          ? `Sem geração de quadros${t.competitivo ? ": jogo competitivo, cada milissegundo conta" : ""}.`
          : "Pode ligar a geração de quadros quando o jogo estiver abaixo do monitor."}
      </span>
    </div>

    {#if r}
      <p class="segura">
        Quem segura aqui é <b>{GARGALO[r.perfil.receita.gargalo] ?? "o jogo"}</b>{r.perfil.fps_antes != null ? ` · ${Math.round(r.perfil.fps_antes)} FPS antes` : ""} · medido em {data(r.perfil.medido_em)}
      </p>
      {#if pecas.length}
        <ul class="pecas">
          {#each pecas as p (p)}<li>{p}</li>{/each}
        </ul>
      {/if}
      <p class="apoio">{r.perfil.receita.explicacao}</p>
      <p class="apoio pequeno">No próximo "Jogar", o Turbo liga esta receita sozinho. Para medir de novo, abra o jogo e use o Turbo.</p>
    {:else}
      <p class="apoio pequeno">Ainda sem receita: com o jogo aberto, o Turbo mede 10 s, descobre quem segura o FPS e guarda aqui.</p>
    {/if}
    {#if recado}<p class="apoio pequeno recado-turbo">{recado}</p>{/if}
  </div>
{/if}

<style>
  .turbo-do-jogo {
    display: grid;
    gap: 10px;
    margin-top: 6px;
    padding: 14px 16px;
    border-radius: 16px;
    background: linear-gradient(180deg, var(--marca-suave), transparent 80%), var(--bloco, var(--superficie-2));
    border: 1px solid var(--linha);
  }
  .cabeca {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .cabeca strong {
    font-family: var(--fonte-titulo);
    font-size: 15px;
  }
  p {
    margin: 0;
  }
  .escolha {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .segura {
    font-size: 13px;
  }
  .pecas {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .pecas li {
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid var(--linha-forte);
    font-size: 12px;
    font-weight: 500;
  }
  .pequeno {
    font-size: 12px;
  }
  .selo[data-tom="perigo"] {
    color: var(--perigo);
  }
  .recado-turbo {
    color: var(--atencao);
  }
</style>
