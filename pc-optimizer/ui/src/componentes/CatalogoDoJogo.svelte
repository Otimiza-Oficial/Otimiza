<script lang="ts">
  // O catálogo do Valorant e do CS2 no destaque do jogo, como o painel do FiveM: a configuração do próprio jogo (o
  // perfil de `configmotor.rs`, só chaves que já existem, com o jogo fechado e desfazer byte a byte), os caches de
  // shader que este jogo usa (os do driver e, no CS2, o da Steam) e o que o anticheat deixa o Otimiza fazer.
  import { onMount } from "svelte";
  import { como } from "../lib/contrato";
  import { commands } from "../lib/tipos";
  import { noTauri } from "../lib/ponte";

  let { jogo }: { jogo: "valorant" | "cs2" } = $props();

  type Previsao = { chave: string; antes: string; depois: string; por_que: string };
  type Config = { id: string; nome: string; arquivo: string | null; aberto: boolean; previsao: Previsao[]; ilegivel: string | null; historico: string; aplicado: boolean };
  type Cache = { id: string; name: string; bytes: number; formatted: string; stale: boolean };

  /** Os caches que este jogo usa: os do driver valem para os dois; o da Steam só para o CS2. */
  const CACHES: Record<typeof jogo, string[]> = {
    valorant: ["nvidia_dx", "amd_dx", "intel", "windows"],
    cs2: ["steam_cs2", "nvidia_dx", "amd_dx", "intel", "windows"],
  };

  const NOME = { valorant: "Valorant", cs2: "Counter-Strike 2" };

  const ANTICHEAT = {
    valorant:
      "O Vanguard vigia tudo que encosta no jogo. Com o Valorant aberto, o Otimiza não mexe no processo dele nem desenha por cima: o Frame Boost e a mira ficam desligados durante a partida. O que vale aqui é a configuração acima, o Turbo deste jogo e o NVIDIA Reflex no menu de vídeo do próprio Valorant.",
    cs2: "O VAC não trata sobreposição como suspeita: o Frame Boost e a mira funcionam no CS2. A integração oficial da Valve, abaixo, mede o FPS só com a rodada valendo.",
  };

  const demo = (): { config: Config; caches: Cache[] } => ({
    config: {
      id: jogo, nome: NOME[jogo], arquivo: "GameUserSettings.ini", aberto: false, ilegivel: null, historico: `config_motor_${jogo}`, aplicado: false,
      previsao: [{ chave: "sg.ShadowQuality", antes: "2", depois: "0", por_que: "Sombras no mínimo: das opções mais caras." }],
    },
    caches: [{ id: "nvidia_dx", name: "Cache do DirectX (NVIDIA)", bytes: 9.1e8, formatted: "910 MB", stale: true }],
  });

  let config = $state<Config | null>(null);
  let caches = $state<Cache[]>([]);
  let ocupado = $state<string | null>(null);
  let recado = $state<string | null>(null);

  async function carregar() {
    if (!noTauri) {
      ({ config, caches } = demo());
      return;
    }
    const [c, s] = await Promise.allSettled([
      como<Config[]>(commands.configMotorPainel()),
      como<{ caches: Cache[] }>(commands.analyzeShaders()),
    ]);
    config = c.status === "fulfilled" ? (c.value.find((x) => x.id === jogo) ?? null) : null;
    caches = s.status === "fulfilled" ? s.value.caches.filter((x) => CACHES[jogo].includes(x.id) && x.bytes > 0) : [];
  }

  onMount(carregar);

  async function fazer(chave: string, acao: () => Promise<string>) {
    ocupado = chave;
    recado = null;
    try {
      recado = await acao();
      await carregar();
    } catch (e) {
      recado = String(e);
    } finally {
      ocupado = null;
    }
  }

  const aplicar = () =>
    fazer("config", async () => {
      if (!noTauri) return "Demonstração: nada foi gravado.";
      const mudou = await como<string[]>(commands.configMotorAplicar(jogo));
      return mudou.length
        ? `Gravei ${mudou.length} ${mudou.length === 1 ? "ajuste" : "ajustes"} na configuração do ${NOME[jogo]}. Abra o jogo e veja no menu de vídeo; o desfazer fica aqui e no Histórico.`
        : "A configuração já estava assim: nada a mudar.";
    });

  const desfazer = (c: Config) =>
    fazer("config", async () => {
      if (noTauri) await como(commands.revertOptimization(c.historico));
      return `A configuração do ${NOME[jogo]} voltou como estava.`;
    });

  const limpar = (x: Cache) => {
    if (!confirm(`Apagar ${x.name} (${x.formatted})? A primeira partida recompila os shaders e pode engasgar; da segunda em diante fica mais liso.`)) return;
    return fazer(x.id, async () =>
      noTauri ? (await como<{ message: string }>(commands.cleanShaderCache(x.id))).message : `Demonstração: ${x.formatted} liberados.`,
    );
  };
</script>

<div class="catalogo">
  <span class="titulo">Catálogo do {NOME[jogo]}</span>

  <div class="bloco">
    <span class="sub">Configuração do jogo</span>
    {#if !config}
      <p class="apoio">Não consegui ler a configuração deste jogo agora.</p>
    {:else if config.ilegivel}
      <p class="aviso">{config.ilegivel}</p>
    {:else if !config.arquivo}
      <p class="apoio">Não achei o arquivo de configuração. Abra o {NOME[jogo]} uma vez e feche: ele cria o arquivo.</p>
    {:else if config.previsao.length === 0}
      <p class="apoio">Já está no perfil leve do Otimiza: nada a mudar.</p>
    {:else}
      <p class="apoio">O perfil leve tira o que pesa na placa sem tirar o que mostra o inimigo. Só muda o que já existe no arquivo do jogo, e volta byte a byte.</p>
      <ul>
        {#each config.previsao as p (p.chave)}
          <li>
            <div>
              <span>{p.chave} · {p.antes} → {p.depois}</span>
              <p class="apoio pequeno">{p.por_que}</p>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
    {#if config?.aberto}<p class="aviso">Feche o {NOME[jogo]} antes: ele reescreve o arquivo ao sair e apagaria a mudança.</p>{/if}
    <div class="acoes">
      {#if config && config.previsao.length > 0}
        <button class="btn" disabled={ocupado !== null || config.aberto} onclick={aplicar}>{ocupado === "config" ? "Gravando…" : "Aplicar perfil leve"}</button>
      {/if}
      {#if config?.aplicado}
        <button class="btn" disabled={ocupado !== null || config.aberto} onclick={() => config && desfazer(config)}>Desfazer</button>
      {/if}
    </div>
  </div>

  <div class="bloco">
    <span class="sub">Cache de shader</span>
    {#if caches.length === 0}
      <p class="apoio">Nenhum cache de shader deste jogo ocupando espaço.</p>
    {:else}
      <p class="apoio">Cache velho, de antes de atualizar o driver ou o jogo, faz o jogo recompilar no meio da partida: são as travadinhas que aparecem do nada.</p>
      <ul>
        {#each caches as x (x.id)}
          <li>
            <div>
              <span>{x.name} · {x.formatted}</span>
              {#if x.stale}<p class="apoio pequeno">Mais velho que o driver: vale limpar.</p>{/if}
            </div>
            <button class="btn" disabled={ocupado !== null || !!config?.aberto} onclick={() => limpar(x)}>{ocupado === x.id ? "Limpando…" : "Limpar"}</button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="bloco">
    <span class="sub">Windows e anticheat</span>
    <p class="apoio">{ANTICHEAT[jogo]}</p>
  </div>

  {#if recado}<p class="recado">{recado}</p>{/if}
</div>

<style>
  .catalogo {
    display: grid;
    gap: 10px;
    padding: 16px 0 0;
    border-top: 1px solid var(--linha);
  }
  .titulo {
    font-weight: 600;
  }
  .bloco {
    display: grid;
    gap: 6px;
  }
  .sub {
    font-size: var(--t-apoio);
    color: var(--tinta-2);
  }
  p {
    margin: 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    font-size: 13px;
  }
  .acoes {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .pequeno {
    font-size: 12px;
  }
  .aviso {
    color: var(--atencao);
    font-size: 13px;
  }
  .recado {
    font-size: 13px;
  }
</style>
