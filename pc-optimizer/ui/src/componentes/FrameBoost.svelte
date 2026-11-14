<script lang="ts">
  import Esqueleto from "./Esqueleto.svelte";
  // O Frame Boost na interface nova (trazido do painel da 3.3, `src/v33/app.ts`, `painelFrameBoost`). Com o jogo
  // aberto, o Rust escolhe o melhor gerador: o do jogo, o do driver (AFMF, Smooth Motion) ou o do Otimiza. Aqui só se
  // mostra: estimado antes de ligar (pelo último FPS nativo medido) e MEDIDO na tela depois; a prova de custo (quanto do
  // FPS o jogo manteve) vem da guarda do Rust. Desligar nunca depende de licença.
  import { onDestroy, onMount } from "svelte";
  import {
    ferramentas,
    motivo,
    NOMES_DA_ESCALA,
    type NivelDeEscala,
    type PainelFrameBoost,
    type Suporte,
  } from "../lib/ferramentas";
  import ComparadorDaEscala from "./ComparadorDaEscala.svelte";

  const CHAVE_MULT = "otimiza-fb-mult";
  const CHAVE_ESCALA = "otimiza-fb-escala";
  const CHAVE_ALVO = "otimiza-fb-alvo";
  const CHAVE_NITIDEZ = "otimiza-fb-nitidez";
  /** FB6: três pontos da faixa do FSR (0 a 2 "stops"); o padrão é o dos exemplos da AMD. */
  const NITIDEZES = [
    { nome: "Suave", stops: 1 },
    { nome: "Padrão", stops: 0.2 },
    { nome: "Forte", stops: 0 },
  ];
  /** Alvos do Auto abaixo do monitor (Plano de refino, FB3): menos quadros gerados, mais placa sobrando para o jogo. */
  const ALVOS = [60, 90, 120, 144, 165, 200, 240];
  const NIVEIS = Object.keys(NOMES_DA_ESCALA) as NivelDeEscala[];

  let p = $state<PainelFrameBoost | null>(null);
  let erro = $state<string | null>(null);
  let ocupado = $state(false);
  let pedido = $state(ler(CHAVE_MULT, "0"));
  let escalaPedida = $state<NivelDeEscala | null>(lerEscala());
  let alvoPedido = $state<number | null>(Number(ler(CHAVE_ALVO, "")) || null);
  let nitidez = $state(Number(ler(CHAVE_NITIDEZ, "0.2")));
  /** Sem a escala permitida nos Ajustes, as opções dela nem aparecem: o Otimiza não muda a resolução do jogo. */
  let escalaPermitida = $state(false);
  let relogio: ReturnType<typeof setInterval> | undefined;

  function ler(chave: string, padrao: string) {
    try {
      return localStorage.getItem(chave) ?? padrao;
    } catch {
      return padrao;
    }
  }
  function lerEscala(): NivelDeEscala | null {
    const v = ler(CHAVE_ESCALA, "");
    return v in NOMES_DA_ESCALA ? (v as NivelDeEscala) : null;
  }
  function gravar(chave: string, v: string | null) {
    try {
      if (v === null) localStorage.removeItem(chave);
      else localStorage.setItem(chave, v);
    } catch {
      // Vale nesta sessão.
    }
  }

  async function carregar() {
    try {
      p = await ferramentas.frameBoost();
      erro = null;
    } catch (e) {
      erro = String(e);
    }
  }

  onMount(() => {
    void carregar();
    // A escolha da pessoa vale desde já: o gerador lê a nitidez a cada quadro.
    void ferramentas.escalaNitidez(nitidez).catch(() => {});
    void ferramentas.escalaPermitida().then((v) => (escalaPermitida = v)).catch(() => {});
    // Ligado pelo gerador do Otimiza o número muda a cada segundo; desligado, basta olhar de vez em quando.
    relogio = setInterval(() => {
      if (!ocupado) void carregar();
    }, 1500);
  });
  onDestroy(() => clearInterval(relogio));

  const a = $derived(p?.ativo ?? null);
  const rec = $derived(p?.recomendado ?? null);
  const doOtimiza = $derived(rec?.recurso === "FrameBoostOtimiza");
  const experimental = $derived(
    rec?.recurso === "FrameBoostSmoothMotion" && (rec ? (motivo(rec.suporte) ?? "") : "").startsWith("Experimental"),
  );
  const liga = $derived(doOtimiza || rec?.recurso === "FrameBoostAfmf" || experimental);
  const escala = $derived(doOtimiza && escalaPermitida ? escalaPedida : null);
  /** O pedido, se couber no monitor; senão o maior que cabe abaixo dele. 1 = só escala (só com a escala ligada). */
  const mult = $derived.by(() => {
    const cabem = p?.multiplicadores ?? [];
    const n = Number(pedido) || 0;
    if (n === 0 || cabem.includes(n) || (n === 1 && escala !== null)) return n;
    return cabem.filter((m) => m <= n).pop() ?? cabem[0] ?? 2;
  });
  /** Os alvos que ficam abaixo deste monitor; o pedido só vale se couber (outro monitor, outro alvo). */
  const alvos = $derived(p?.hz ? ALVOS.filter((x) => x < (p?.hz ?? 0)) : []);
  const alvo = $derived(alvoPedido != null && alvos.includes(alvoPedido) ? alvoPedido : null);
  const estimativa = $derived.by(() => {
    if (!p || mult === 1) return null;
    const dois = p.estimativas.find((e) => e.multiplicador === 2);
    if (mult === 0) {
      if (!dois || !p.hz) return null;
      const teto = alvo ?? p.hz;
      return { minimo: Math.min(teto, (dois.faixa.minimo / 2) * 6), maximo: Math.min(teto, (dois.faixa.maximo / 2) * 6) };
    }
    return p.estimativas.find((e) => e.multiplicador === mult)?.faixa ?? null;
  });
  const medido = $derived(
    a && (a.fps_na_tela.origem === "Medido" || a.fps_na_tela.origem === "Estimado") ? a.fps_na_tela.valor : null,
  );
  const soEscalaLigada = $derived(!!a?.escala && !a.automatico && a.multiplicador <= 1);

  const n = (v: number, casas = 0) => v.toLocaleString("pt-BR", { maximumFractionDigits: casas, minimumFractionDigits: casas });

  function selo(s: Suporte) {
    if (s.estado === "Suportado") return { texto: "funciona aqui", tom: "ok" };
    if (s.estado === "Parcial") return { texto: "com condição", tom: "atencao" };
    if (s.estado === "PrecisaAtualizar") return { texto: "atualize o driver", tom: "atencao" };
    return { texto: "indisponível", tom: "neutro" };
  }

  const linha = $derived.by(() => {
    if (!a) return "";
    if (a.pelo_driver) return a.nota ?? "";
    if (a.situacao === "ProcurandoJanela") return "Ajustando à janela do jogo…";
    if (a.situacao === "SemQuadros") return "O jogo parou de desenhar: a sobreposição some até ele voltar.";
    const tamanho = a.escala && a.jogo_px && a.tela_px ? `jogo ${a.jogo_px[0]}×${a.jogo_px[1]} → tela ${a.tela_px[0]}×${a.tela_px[1]} · ` : "";
    const modo = soEscalaLigada ? "só escala, sem gerar quadros" : a.automatico ? `Auto (agora ${a.multiplicador}×)` : `${a.multiplicador}×`;
    const atraso = a.atraso_ms.valor !== null ? ` · atraso +${n(a.atraso_ms.valor, 1)} ms (estimado)` : "";
    const placa = a.placa_pct != null ? ` · usa ${n(a.placa_pct, 1)}% da placa (medido)` : "";
    return `${tamanho}${modo} · monitor de ${a.hz} Hz${atraso}${placa}`;
  });

  const prova = $derived.by(() => {
    if (!a || a.pelo_driver || a.situacao !== "Gerando" || a.fps_do_jogo === null) return "";
    if (soEscalaLigada) return `Jogo a ${n(a.fps_do_jogo)} fps`;
    return `Jogo a ${n(a.fps_do_jogo)} fps · ${
      a.manteve_pct !== null
        ? `manteve ${n(a.manteve_pct, 1)}% do FPS com o Frame Boost (medido em ${a.comparacoes} ${a.comparacoes === 1 ? "comparação" : "comparações"})`
        : "conferindo se o jogo perde FPS com o Frame Boost…"
    }`;
  });

  const avisos = $derived.by(() => {
    const l: string[] = [];
    if (p?.aviso) l.push(p.aviso);
    if (a?.integrada) l.push("Este notebook liga o monitor na placa integrada, e o gerador roda nela. Se o jogo perder FPS, ele desliga sozinho.");
    if (p?.hz != null && p.hz <= 75 && doOtimiza)
      l.push(`Monitor de ${p.hz} Hz: o Frame Boost nunca mostra mais quadros que o monitor. Se ele aceita mais Hz, ajuste na área Tela primeiro.`);
    return l;
  });

  async function agir(f: () => Promise<PainelFrameBoost>) {
    ocupado = true;
    erro = null;
    try {
      p = await f();
    } catch (e) {
      erro = String(e);
    } finally {
      ocupado = false;
    }
  }

  function escolherMult(m: number) {
    pedido = String(m);
    gravar(CHAVE_MULT, pedido);
  }
  function escolherNitidez(stops: number) {
    nitidez = stops;
    gravar(CHAVE_NITIDEZ, String(stops));
    void ferramentas.escalaNitidez(stops).catch(() => {});
  }
  function escolherAlvo(v: number | null) {
    alvoPedido = v;
    gravar(CHAVE_ALVO, v == null ? null : String(v));
  }
  function escolherEscala(e: NivelDeEscala | null) {
    escalaPedida = e;
    gravar(CHAVE_ESCALA, e);
  }
</script>

<section class="secao fb" class:aceso={!!a}>
  <div class="topo">
    <h2>Frame Boost</h2>
    {#if p && rec}<span class="gerador">{rec.nome}</span>{/if}
    {#if a}<span class="ligado"><span class="ponto" data-tom="ok"></span>Ligado</span>{/if}
  </div>

  {#if !p}
    {#if erro}<p class="apoio">{erro}</p>{:else}<Esqueleto />{/if}
  {:else}
    {#if rec && p.jogo}
      <p class="apoio">{motivo(rec.suporte) ?? (doOtimiza ? "Funciona por fora do jogo, sem injetar nada. Acrescenta um pouco de atraso." : "Funciona neste jogo e nesta placa.")}</p>
    {/if}
    {#if !rec}
      <div class="vazio">
        <strong>{p.jogo ? "Nenhum gerador funciona aqui" : "Abra um jogo"}</strong>
        <p class="apoio">
          {p.jogo
            ? "Nem o jogo, nem o driver, nem o gerador do Otimiza servem neste jogo com esta placa."
            : "Com o jogo aberto, o Otimiza escolhe o melhor gerador de quadros: o do jogo, depois o do driver, depois o dele."}
        </p>
      </div>
    {:else}
      <!-- O herói: o número que chega à tela, e à direita o que se escolhe -->
      <div class="palco">
        <div class="heroi">
          <span class="rotulo">FPS na tela</span>
          {#if a}
            <strong class="numero gigante">{medido !== null ? n(medido) : "—"}</strong>
            <div class="selos">
              {#if medido === null}
                <span class="selo">{a.pelo_driver ? "falta medir" : "medindo"}</span>
              {:else if a.fps_na_tela.origem === "Estimado"}
                <span class="selo estimado" title="Quadros apresentados pelo gerador. Abra o Otimiza como administrador para medir o que chega à tela.">estimado</span>
              {:else}
                <span class="selo medido">medido na tela</span>
              {/if}
              {#if a.leve}<span class="selo">modo leve</span>{/if}
              {#if a.escala}<span class="selo">escala {NOMES_DA_ESCALA[a.escala]}</span>{/if}
            </div>
            {#if linha}<p class="apoio">{linha}</p>{/if}
            {#if prova}<p class="apoio">{prova}</p>{/if}
          {:else if estimativa}
            {@const de = n(estimativa.minimo)}
            {@const ate = n(estimativa.maximo)}
            <strong class="numero gigante">{de === ate ? de : `${de}–${ate}`}</strong>
            <div class="selos"><span class="selo estimado">estimado</span><span class="apoio">{mult === 0 ? "no Auto" : `com ${mult}×`}</span></div>
            <p class="apoio">Pelo último FPS nativo deste jogo. O número de verdade aparece medido quando ligar.</p>
          {:else}
            <strong class="numero gigante apagado">—</strong>
            <p class="apoio">
              {doOtimiza && mult === 1
                ? "Só escala: o jogo renderiza menor e o Otimiza amplia. O FPS a mais é do próprio jogo."
                : doOtimiza
                  ? "Jogue um minuto para o Otimiza medir o FPS nativo e estimar o ganho. Dá para ligar mesmo assim."
                  : (motivo(rec.suporte) ?? "Funciona neste jogo e nesta placa.")}
            </p>
          {/if}
        </div>

        <div class="controles">
          {#if doOtimiza && !a}
            {#if escalaPermitida}
              <div class="escolha">
                <span class="rotulo">Escala</span>
                <div class="segmento" role="radiogroup" aria-label="Escala">
                  <button role="radio" aria-checked={escala === null} onclick={() => escolherEscala(null)}>Desligada</button>
                  {#each NIVEIS as nivel (nivel)}
                    <button role="radio" aria-checked={escala === nivel} onclick={() => escolherEscala(nivel)}>{NOMES_DA_ESCALA[nivel]}</button>
                  {/each}
                </div>
              </div>
            {/if}
            <div class="escolha">
              <span class="rotulo">Quadros gerados</span>
              <div class="segmento" role="radiogroup" aria-label="Multiplicador">
                {#if escala}<button role="radio" aria-checked={mult === 1} onclick={() => escolherMult(1)}>Só escala</button>{/if}
                <button role="radio" aria-checked={mult === 0} onclick={() => escolherMult(0)}>Auto</button>
                {#each p.multiplicadores as m (m)}
                  <button role="radio" aria-checked={mult === m} onclick={() => escolherMult(m)}>{m}×</button>
                {/each}
              </div>
              <p class="dica">{mult === 0 ? "Enche o monitor. Para jogo competitivo, o 2× tem menos atraso." : mult === 1 ? "Sem gerar quadros: só a escala." : `${mult} quadros na tela para cada um do jogo.`}</p>
            </div>
            {#if mult === 0 && alvos.length}
              <div class="escolha">
                <span class="rotulo">FPS na tela</span>
                <div class="segmento" role="radiogroup" aria-label="FPS alvo">
                  <button role="radio" aria-checked={alvo === null} onclick={() => escolherAlvo(null)}>Monitor ({p.hz})</button>
                  {#each alvos as v (v)}
                    <button role="radio" aria-checked={alvo === v} onclick={() => escolherAlvo(v)}>{v}</button>
                  {/each}
                </div>
                <p class="dica">{alvo ? `Menos quadros gerados: gasta menos placa e o jogo fica com mais FPS de verdade.` : "Abaixo do monitor, o gerador gasta menos placa."}</p>
              </div>
            {/if}
          {/if}

          {#if doOtimiza && (escala || a?.escala)}
            <div class="escolha">
              <span class="rotulo">Nitidez da escala</span>
              <div class="segmento" role="radiogroup" aria-label="Nitidez da escala">
                {#each NITIDEZES as o (o.nome)}
                  <button role="radio" aria-checked={nitidez === o.stops} onclick={() => escolherNitidez(o.stops)}>{o.nome}</button>
                {/each}
              </div>
            </div>
          {/if}

          <div class="acoes">
            {#if a}
              <button class="btn" disabled={ocupado} onclick={() => agir(ferramentas.frameBoostDesligar)}>Desligar Frame Boost</button>
              {#if a.pelo_driver}
                <button class="btn" disabled={ocupado} onclick={() => agir(ferramentas.frameBoostMedir)}>{ocupado ? "Medindo 6 s…" : "Medir agora"}</button>
              {/if}
            {:else}
              <button class="btn primario grande" disabled={ocupado || !liga} onclick={() => agir(() => ferramentas.frameBoostLigar(mult, escala, mult === 0 ? alvo : null))}>
                {experimental ? "Ligar (experimental)" : liga ? "Ligar Frame Boost" : "Liga pelo painel do driver"}
              </button>
            {/if}
            {#if doOtimiza}<span class="atalho" title="Desliga de dentro do jogo"><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>G</kbd> desliga no jogo</span>{/if}
          </div>
        </div>
      </div>

      {#if doOtimiza && !a && !escalaPermitida}
        <p class="apoio pequeno">Sem escala: o Frame Boost só gera quadros e não muda a resolução do jogo. Para usar a escala, ligue "Permitir a escala" nos Ajustes.</p>
      {/if}
      {#if doOtimiza && !a && escala}
        {@const o = p.escalas?.find((x) => x.nivel === escala)}
        <p class="apoio pequeno">
          {o && p.monitor
            ? `O jogo vai para ${o.largura}×${o.altura} e o Otimiza amplia para ${p.monitor[0]}×${p.monitor[1]}; ao desligar, a janela volta como estava. O jogo precisa estar em janela sem bordas.`
            : "O Otimiza põe o jogo numa janela menor e amplia para a tela inteira. O jogo precisa estar em janela sem bordas."}
        </p>
      {/if}
    {/if}

    {#if a?.escala && !a.pelo_driver && a.situacao === "Gerando"}<ComparadorDaEscala />{/if}

    {#if doOtimiza && (p.base || p.nota_da_base)}
      <div class="linha">
        <div>
          <span>FPS base{#if p.base}&nbsp;<b>{p.base.fps}</b> · {p.base.multiplicador}× fecham o monitor
              <span class="selo" class:medido={p.base.valendo}>{p.base.valendo ? "valendo" : "vale ao reabrir o jogo"}</span>{/if}</span>
          <p class="apoio">{p.nota_da_base ?? "Base travada numa fração do monitor: cada quadro real ganha sempre o mesmo número de gerados."}</p>
        </div>
        {#if p.base}<button class="btn" disabled={ocupado} onclick={() => agir(ferramentas.frameBoostDestravarBase)}>Destravar</button>{/if}
      </div>
    {/if}
    {#if doOtimiza && p.base_solta && !p.base}
      <div class="linha">
        <div>
          <span>FPS base solto</span>
          <p class="apoio">Você destravou: o Frame Boost não trava a base deste jogo sozinho.</p>
        </div>
        {#if a && !a.pelo_driver}<button class="btn" disabled={ocupado} onclick={() => agir(ferramentas.frameBoostTravarBase)}>Travar de novo</button>{/if}
      </div>
    {/if}

    {#if !a && p.ultima_sessao}
      {@const u = p.ultima_sessao}
      <div class="linha">
        <div>
          <span>Da última vez · {n(u.minutos, 1)} min</span>
          <p class="apoio">
            Jogo a {n(u.fps_do_jogo)} fps → {u.fps_na_tela.valor !== null ? n(u.fps_na_tela.valor) : "—"} na tela{u.manteve_pct !== null
              ? ` · o jogo manteve ${n(u.manteve_pct, 1)}% do FPS`
              : ""}{u.desligou_por_perda ? " · desligou sozinho: o jogo perdia FPS" : ""}
          </p>
        </div>
        <span class="selo" class:medido={u.fps_na_tela.origem === "Medido"}>{u.fps_na_tela.origem === "Medido" ? "medido" : "estimado"}</span>
      </div>
    {/if}

    {#each avisos as t, i (i)}<p class="aviso">{t}</p>{/each}
    {#if erro}<p class="erro">{erro}</p>{/if}

    {#if p.opcoes.length}
      <div class="outros">
        <span class="rotulo">Outros geradores</span>
        <ul class="opcoes">
          {#each p.opcoes as r (r.recurso)}
            {@const s = selo(r.suporte)}
            <li title={motivo(r.suporte) ?? undefined}>
              <span>{r.nome}</span>
              <span class="selo" data-tom={s.tom}>{s.texto}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  {/if}
</section>

<style>
  .fb {
    display: grid;
    gap: 16px;
  }
  /* Ligado: uma luz da marca acende atrás do número. */
  .fb.aceso {
    isolation: isolate;
  }
  .fb.aceso::before {
    content: "";
    position: absolute;
    z-index: -1;
    inset: 0;
    border-radius: inherit;
    background: radial-gradient(60% 70% at 18% 30%, var(--marca-suave), transparent 70%);
    pointer-events: none;
  }
  .topo {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .topo h2 {
    flex: 1;
  }
  .gerador {
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--bloco, var(--superficie-2));
    border: 1px solid var(--linha);
    color: var(--tinta-2);
    font-size: 12px;
    font-weight: 500;
  }
  .ligado {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--ok) 14%, transparent);
    color: var(--ok);
    font-size: 12px;
    font-weight: 600;
  }
  p {
    margin: 0;
  }
  .vazio {
    display: grid;
    gap: 6px;
    padding: 6px 0 2px;
  }
  .palco {
    position: relative;
    display: grid;
    grid-template-columns: minmax(220px, 0.9fr) 1.3fr;
    gap: 28px;
    align-items: start;
  }
  .heroi {
    display: grid;
    gap: 8px;
    align-content: start;
  }
  .gigante {
    font-size: 84px;
    line-height: 0.92;
    font-weight: 700;
    letter-spacing: -0.045em;
  }
  .apagado {
    color: var(--tinta-3);
  }
  .selos {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
  }
  .controles {
    display: grid;
    gap: 16px;
    padding: 16px 18px;
    border-radius: 16px;
    background: var(--bloco, var(--superficie-2));
    border: 1px solid var(--linha);
  }
  .escolha {
    display: grid;
    gap: 8px;
    justify-items: start;
  }
  .rotulo {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--tinta-3);
  }
  .dica {
    font-size: 12px;
    color: var(--tinta-2);
  }
  .acoes {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
    padding-top: 4px;
  }
  .grande {
    min-height: 44px;
    padding: 0 22px;
    font-weight: 600;
  }
  .atalho {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--tinta-2);
  }
  kbd {
    display: inline-grid;
    place-items: center;
    min-width: 24px;
    height: 22px;
    padding: 0 6px;
    border-radius: 6px;
    border: 1px solid var(--linha-forte);
    border-bottom-width: 2px;
    font-family: var(--fonte-texto);
    font-size: 11px;
    font-weight: 600;
    color: var(--tinta);
  }
  .linha {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    padding-top: 12px;
    border-top: 1px solid var(--linha);
  }
  .pequeno {
    font-size: 12px;
  }
  .aviso {
    color: var(--atencao);
    font-size: 13px;
  }
  .erro {
    color: var(--perigo);
    font-size: 13px;
  }
  .outros {
    display: grid;
    gap: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--linha);
  }
  .opcoes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .opcoes li {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 8px 6px 12px;
    border-radius: 999px;
    border: 1px solid var(--linha);
    font-size: 12px;
  }
  @media (max-width: 1100px) {
    .palco {
      grid-template-columns: 1fr;
    }
  }
</style>
