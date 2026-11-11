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
  const NIVEIS = Object.keys(NOMES_DA_ESCALA) as NivelDeEscala[];

  let p = $state<PainelFrameBoost | null>(null);
  let erro = $state<string | null>(null);
  let ocupado = $state(false);
  let pedido = $state(ler(CHAVE_MULT, "0"));
  let escalaPedida = $state<NivelDeEscala | null>(lerEscala());
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
  const escala = $derived(doOtimiza ? escalaPedida : null);
  /** O pedido, se couber no monitor; senão o maior que cabe abaixo dele. 1 = só escala (só com a escala ligada). */
  const mult = $derived.by(() => {
    const cabem = p?.multiplicadores ?? [];
    const n = Number(pedido) || 0;
    if (n === 0 || cabem.includes(n) || (n === 1 && escala !== null)) return n;
    return cabem.filter((m) => m <= n).pop() ?? cabem[0] ?? 2;
  });
  const estimativa = $derived.by(() => {
    if (!p || mult === 1) return null;
    const dois = p.estimativas.find((e) => e.multiplicador === 2);
    if (mult === 0) {
      if (!dois || !p.hz) return null;
      return { minimo: Math.min(p.hz, (dois.faixa.minimo / 2) * 6), maximo: Math.min(p.hz, (dois.faixa.maximo / 2) * 6) };
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
  function escolherEscala(e: NivelDeEscala | null) {
    escalaPedida = e;
    gravar(CHAVE_ESCALA, e);
  }
</script>

<section class="secao fb">
  <div class="topo">
    <h2>Frame Boost</h2>
    {#if a}<span class="ligado"><span class="ponto" data-tom="ok"></span>ligado</span>{/if}
  </div>

  {#if !p}
    {#if erro}<p class="apoio">{erro}</p>{:else}<Esqueleto />{/if}
  {:else}
    <p class="destaque">{#if rec}<span class="apoio">Gerador escolhido</span> {rec.nome}{:else}{p.jogo ? "Nenhum gerador funciona aqui" : "Abra um jogo"}{/if}</p>
    <p class="apoio">
      {#if !p.jogo}
        Com o jogo aberto, o Otimiza escolhe o melhor gerador de quadros: o do jogo, depois o do driver, depois o dele.
      {:else if rec}
        {motivo(rec.suporte) ?? (doOtimiza ? "Funciona por fora do jogo, sem injetar nada. Acrescenta um pouco de atraso." : "Funciona neste jogo e nesta placa.")}
      {:else}
        Nem o jogo, nem o driver, nem o gerador do Otimiza servem neste jogo com esta placa.
      {/if}
    </p>

    {#if rec}
      <div class="numero-grande">
        {#if a}
          <strong class="numero">{medido !== null ? n(medido) : "—"}</strong>
          <span>{medido !== null && a.hz > 0 && medido > a.hz * 1.03 ? "FPS" : "FPS na tela"}</span>
          {#if medido === null}
            <span class="selo">{a.pelo_driver ? "falta medir" : "medindo"}</span>
          {:else if a.fps_na_tela.origem === "Estimado"}
            <span class="selo estimado" title="Quadros apresentados pelo gerador. Abra o Otimiza como administrador para medir o que chega à tela.">estimado</span>
          {:else}
            <span class="selo medido">medido na tela</span>
          {/if}
          {#if a.leve}<span class="selo">modo leve</span>{/if}
          {#if a.escala}<span class="selo">escala {NOMES_DA_ESCALA[a.escala]}</span>{/if}
        {:else if estimativa}
          {@const de = n(estimativa.minimo)}
          {@const ate = n(estimativa.maximo)}
          <strong class="numero">{de === ate ? de : `${de}–${ate}`}</strong>
          <span>{mult === 0 ? "FPS na tela no Auto" : `FPS na tela com ${mult}×`}</span>
          <span class="selo estimado">estimado</span>
        {/if}
      </div>
      {#if a}
        {#if linha}<p class="apoio">{linha}</p>{/if}
        {#if prova}<p class="apoio">{prova}</p>{/if}
      {:else if doOtimiza && mult === 1}
        <p class="apoio">Só escala: o jogo renderiza menor e o Otimiza amplia. O FPS a mais é do próprio jogo, sem o atraso da geração.</p>
      {:else if estimativa}
        <p class="apoio">Pelo último FPS nativo medido deste jogo. O número de verdade aparece medido quando ligar.</p>
      {:else if doOtimiza}
        <p class="apoio">Jogue um minuto para o Otimiza medir o FPS nativo e estimar o ganho. Dá para ligar mesmo assim.</p>
      {/if}
    {/if}

    {#if doOtimiza && !a}
      <div class="escolha">
        <span class="rotulo">Escala</span>
        <div class="segmento" role="radiogroup" aria-label="Escala">
          <button role="radio" aria-checked={escala === null} onclick={() => escolherEscala(null)}>Desligada</button>
          {#each NIVEIS as nivel (nivel)}
            <button role="radio" aria-checked={escala === nivel} onclick={() => escolherEscala(nivel)}>{NOMES_DA_ESCALA[nivel]}</button>
          {/each}
        </div>
        <p class="apoio pequeno">
          {#if !escala}
            O jogo renderiza numa resolução menor e o Otimiza amplia para o monitor (FSR 1): é FPS real a mais, não só quadro gerado.
          {:else}
            {@const o = p.escalas?.find((x) => x.nivel === escala)}
            {o && p.monitor
              ? `O Otimiza põe o jogo em ${o.largura}×${o.altura} e amplia para ${p.monitor[0]}×${p.monitor[1]}; ao desligar, a janela volta como estava. O jogo precisa estar em janela sem bordas.`
              : "O Otimiza põe o jogo numa janela menor e amplia para a tela inteira. O jogo precisa estar em janela sem bordas."}
          {/if}
        </p>
      </div>
      <div class="escolha">
        <span class="rotulo">Quadros gerados</span>
        <div class="segmento" role="radiogroup" aria-label="Multiplicador">
          {#if escala}<button role="radio" aria-checked={mult === 1} onclick={() => escolherMult(1)}>Só escala</button>{/if}
          <button role="radio" aria-checked={mult === 0} onclick={() => escolherMult(0)}>Auto</button>
          {#each p.multiplicadores as m (m)}
            <button role="radio" aria-checked={mult === m} onclick={() => escolherMult(m)}>{m}×</button>
          {/each}
        </div>
        {#if mult === 0}
          <p class="apoio pequeno">
            Auto enche o monitor: com o jogo a 60 fps num de 180 Hz, a tela recebe 180. Mais liso, mas segura o quadro real
            por cerca de um quadro do jogo; para jogo competitivo, o 2× tem menos atraso.
          </p>
        {/if}
      </div>
    {/if}

    {#if rec}
      <div class="acoes">
        {#if a}
          <button class="btn" disabled={ocupado} onclick={() => agir(ferramentas.frameBoostDesligar)}>Desligar Frame Boost</button>
          {#if a.pelo_driver}
            <button class="btn" disabled={ocupado} onclick={() => agir(ferramentas.frameBoostMedir)}>{ocupado ? "Medindo 6 s…" : "Medir agora"}</button>
          {/if}
        {:else}
          <button class="btn primario" disabled={ocupado || !liga} onclick={() => agir(() => ferramentas.frameBoostLigar(mult, escala))}>
            {experimental ? "Ligar (experimental)" : liga ? "Ligar Frame Boost" : "Liga pelo painel do driver"}
          </button>
        {/if}
        {#if doOtimiza}<span class="apoio pequeno">Ctrl+Alt+G desliga de dentro do jogo</span>{/if}
      </div>
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
      <ul class="opcoes">
        {#each p.opcoes as r (r.recurso)}
          {@const s = selo(r.suporte)}
          <li>
            <div><span>{r.nome}</span>{#if motivo(r.suporte)}<p class="apoio pequeno">{motivo(r.suporte)}</p>{/if}</div>
            <span class="selo" data-tom={s.tom}>{s.texto}</span>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<style>
  .fb {
    display: grid;
    gap: 10px;
  }
  .topo {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .topo h2 {
    flex: 1;
  }
  .ligado {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
  .destaque {
    font-weight: 500;
    margin: 0;
  }
  p {
    margin: 0;
  }
  .numero-grande {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }
  .numero-grande .numero {
    font-size: 56px;
    line-height: 1;
    font-weight: 600;
    letter-spacing: -0.03em;
  }
  .numero-grande > span:not(.selo) {
    color: var(--tinta-2);
  }
  .escolha {
    display: grid;
    gap: 6px;
  }
  .rotulo {
    font-size: var(--t-apoio);
    color: var(--tinta-2);
  }
  .acoes {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  .linha {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    padding-top: 10px;
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
  .opcoes {
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  .opcoes li {
    display: flex;
    gap: 12px;
    justify-content: space-between;
    align-items: center;
    font-size: 13px;
  }
</style>
