<script lang="ts">
  import { como } from "../lib/contrato";
  import { commands } from "../lib/tipos";
  import Esqueleto from "./Esqueleto.svelte";
  // A BIOS (trazida da interface antiga, `src/bios.ts`). O Otimiza NÃO grava na BIOS: em placa de consumo um erro ali
  // deixa a placa sem ligar. Mostra o que o Windows deixa ler (ficha), os defeitos conhecidos (microcódigo Intel,
  // boot lento), o que olhar na BIOS em ordem de risco, e um "antes e depois": a foto guardada neste PC antes de
  // reiniciar na BIOS, comparada na volta. Sem foto, sem comparação, e a tela diz.
  import { onMount } from "svelte";
    import { noTauri } from "../lib/ponte";

  type Defeito =
    | { tipo: "MicrocodigoIntelAntigo"; atual: number }
    | { tipo: "BootLentoAm5"; segundos: number }
    | { tipo: "BootLento"; segundos: number };
  type Ficha = {
    leitura: { cpu: string | null; microcodigo: number | null; integridade_de_memoria: boolean | null; tempo_da_bios_s: number | null; lacunas: string[] };
    defeitos: Defeito[];
    notebook: boolean;
  };
  type Passo = { fase: string; titulo: string; onde: string; o_que_faz: string; risco_e_volta: string; medido_aqui: boolean };
  type Firmware = {
    leitura: { placa_mae: string | null; versao_da_bios: string | null; data_da_bios: string | null; uefi: boolean | null; secure_boot: boolean | null; lacunas: string[] };
    passos: Passo[];
  };
  type Memoria = { slots: number | null; pentes_gb: number[]; canais: number; mhz: number | null };
  type Foto = { quando: number; valores: Record<string, string> };

  const CHAVE_DA_FOTO = "otimiza.bios.foto";
  const NAO_LIDO = "não deu para ler";
  const NAO_REGISTRADO = "não registrado";

  let ficha = $state<Ficha | null>(null);
  let firmware = $state<Firmware | null>(null);
  let memoria = $state<Memoria | null>(null);
  let erro = $state<string | null>(null);
  let lido = $state(false);
  let foto = $state<Foto | null>(lerFoto());
  let recado = $state<string | null>(null);
  let reiniciando = $state(false);

  const demo = {
    ficha: { leitura: { cpu: "Intel Core i5-13600K", microcodigo: 0x129, integridade_de_memoria: true, tempo_da_bios_s: 14.2, lacunas: [] }, defeitos: [{ tipo: "MicrocodigoIntelAntigo", atual: 0x129 }], notebook: false } as Ficha,
    firmware: {
      leitura: { placa_mae: "ASUS PRIME B760M-A", versao_da_bios: "1611", data_da_bios: "20230512000000.000000+000", uefi: true, secure_boot: true, lacunas: [] },
      passos: [
        { fase: "Memória", titulo: "Perfil de memória (XMP/EXPO)", onde: "Ai Tweaker › Ai Overclock Tuner › XMP I", o_que_faz: "Põe a memória na velocidade da caixa (hoje ela roda abaixo).", risco_e_volta: "Se o PC não ligar, ele volta sozinho ao padrão depois de três tentativas.", medido_aqui: true },
        { fase: "Placa de vídeo", titulo: "Resizable BAR", onde: "Advanced › PCI Subsystem › Re-Size BAR Support", o_que_faz: "Deixa o processador ver a memória da placa inteira de uma vez.", risco_e_volta: "Baixo. Precisa do modo UEFI e do CSM desligado.", medido_aqui: false },
      ],
    } as Firmware,
    memoria: { slots: 4, pentes_gb: [8, 8], canais: 2, mhz: 4800 } as Memoria,
  };

  function lerFoto(): Foto | null {
    try {
      const t = localStorage.getItem(CHAVE_DA_FOTO);
      return t ? (JSON.parse(t) as Foto) : null;
    } catch {
      return null;
    }
  }

  onMount(async () => {
    if (!noTauri) {
      ({ ficha, firmware, memoria } = demo);
      lido = true;
      return;
    }
    const [f, fw, m] = await Promise.allSettled([
      como<Ficha>(commands.fichaDaBios()),
      como<Firmware>(commands.passoAPassoDaBios()),
      como<Memoria>(commands.memoriaInstalada()),
    ]);
    ficha = f.status === "fulfilled" ? f.value : null;
    firmware = fw.status === "fulfilled" ? fw.value : null;
    memoria = m.status === "fulfilled" ? m.value : null;
    const falha = [f, fw].find((r) => r.status === "rejected") as PromiseRejectedResult | undefined;
    erro = falha ? String(falha.reason) : null;
    lido = true;
  });

  const simNao = (v: boolean | null | undefined, sim: string, nao: string) => (v === true ? sim : v === false ? nao : NAO_LIDO);
  const dataDaBios = (b: string | null) => {
    const m = b?.match(/^(\d{4})(\d{2})(\d{2})/);
    return m ? `${m[3]}/${m[2]}/${m[1]}` : b;
  };
  const hex = (n: number) => `0x${n.toString(16).toUpperCase()}`;

  const linhas = $derived.by<[string, string][]>(() => {
    const l = firmware?.leitura;
    const c = ficha?.leitura;
    const r: [string, string][] = [
      ["Placa-mãe", l?.placa_mae ?? NAO_LIDO],
      ["Versão da BIOS", l?.versao_da_bios ? `${l.versao_da_bios}${l.data_da_bios ? ` · ${dataDaBios(l.data_da_bios)}` : ""}` : NAO_LIDO],
      ["Modo de inicialização", l ? simNao(l.uefi, "UEFI", "Legacy (CSM)") : NAO_LIDO],
      ["Secure Boot", l ? simNao(l.secure_boot, "ligado", "desligado") : NAO_LIDO],
      ["Processador", c?.cpu ?? NAO_LIDO],
    ];
    if (c?.microcodigo != null) r.push(["Microcódigo", hex(c.microcodigo)]);
    r.push(["Integridade de Memória", c ? simNao(c.integridade_de_memoria, "ligada", "desligada") : NAO_LIDO]);
    r.push([
      "Memória",
      memoria
        ? `${memoria.pentes_gb.reduce((a, b) => a + b, 0)} GB · ${memoria.mhz ? `${memoria.mhz} MHz` : "velocidade não lida"} · ${memoria.canais === 1 ? "canal único" : `${memoria.canais} canais`}`
        : NAO_LIDO,
    ]);
    r.push(["Tempo da placa no boot", c?.tempo_da_bios_s != null ? `${c.tempo_da_bios_s.toFixed(1).replace(".", ",")} s` : NAO_REGISTRADO]);
    return r;
  });
  const valores = $derived(Object.fromEntries(linhas));
  const lacunas = $derived([...(firmware?.leitura.lacunas ?? []), ...(ficha?.leitura.lacunas ?? [])]);
  const conferido = $derived(!!ficha?.leitura.cpu);

  function fraseDoDefeito(d: Defeito): string {
    switch (d.tipo) {
      case "MicrocodigoIntelAntigo":
        return `O microcódigo do processador é o ${hex(d.atual)}. A Intel publicou o 0x12F como correção da instabilidade que degrada com o tempo os processadores de mesa da 13ª e 14ª geração (Raptor Lake), como este. Ele chega por atualização de BIOS: procure na página oficial da sua placa uma versão que cite o microcódigo 0x12F. O Otimiza não atualiza BIOS.`;
      case "BootLentoAm5":
        return `A placa-mãe levou ${d.segundos} s no último boot. Em placas AM5 com o perfil EXPO ligado, o mais comum é a placa treinar a memória a cada boot. A opção Memory Context Restore reaproveita o treino e costuma cortar esse tempo. Se o PC ficar instável depois de ligar, desligue de novo.`;
      case "BootLento":
        return `A placa-mãe levou ${d.segundos} s no último boot. Vale conferir na BIOS se o Fast Boot está ligado e se o modo CSM (Legacy) está desligado.`;
    }
  }

  /** O que mudou desde a foto, só entre itens lidos dos dois lados ("não deu para ler" igual não é "nada mudou"). */
  const comparacao = $derived.by(() => {
    if (!foto) return null;
    const ok = (v: string | undefined) => v !== undefined && v !== NAO_LIDO && v !== NAO_REGISTRADO;
    const comparaveis = Object.entries(valores).filter(([k, v]) => ok(foto!.valores[k]) && ok(v));
    return {
      quando: new Date(foto.quando).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" }),
      mudou: comparaveis.filter(([k, v]) => foto!.valores[k] !== v).map(([k, v]) => ({ k, antes: foto!.valores[k], depois: v })),
      fora: Object.keys(valores).length - comparaveis.length,
    };
  });

  function texto(): string {
    const p = ["Ficha do firmware (Otimiza)", ...linhas.map(([k, v]) => `${k}: ${v}`)];
    const d = ficha?.defeitos ?? [];
    p.push("", "Defeitos conhecidos:", ...(d.length ? d.map((x) => "- " + fraseDoDefeito(x)) : [conferido ? "- nenhum" : "- não conferido"]));
    if (firmware?.passos.length) {
      p.push("", "O que olhar na BIOS, em ordem de risco:");
      for (const x of firmware.passos) p.push(`- ${x.titulo}${x.medido_aqui ? " (medido aqui)" : ""}`, `  Onde: ${x.onde}`, `  O que faz: ${x.o_que_faz}`, `  Risco e volta: ${x.risco_e_volta}`);
    }
    if (lacunas.length) p.push("", "Não deu para ler:", ...lacunas.map((l) => "- " + l));
    return p.join("\n");
  }

  function guardarFoto(): boolean {
    try {
      const f = { quando: Date.now(), valores: { ...valores } };
      localStorage.setItem(CHAVE_DA_FOTO, JSON.stringify(f));
      foto = f;
      return true;
    } catch {
      return false;
    }
  }

  async function copiar() {
    try {
      await navigator.clipboard.writeText(texto());
      recado = "Copiada. Cole no Discord ou mande ao suporte.";
    } catch {
      recado = "Não deu para copiar.";
    }
  }

  async function salvar() {
    try {
      recado = noTauri ? `Salva em ${await como<string>(commands.salvarFichaDaBios(texto()))}` : "Salva em Documentos\\ficha-da-bios.txt";
    } catch (e) {
      recado = String(e);
    }
  }

  async function reiniciarNaBios() {
    if (!confirm("O PC vai reiniciar direto na tela da BIOS em 5 segundos. Salve o que estiver aberto. Continuar?")) return;
    guardarFoto();
    reiniciando = true;
    try {
      if (noTauri) await commands.reiniciarNaBios();
      recado = "Reiniciando na BIOS…";
    } catch (e) {
      reiniciando = false;
      recado = String(e);
    }
  }
</script>

{#if !lido}
  <Esqueleto rotulo="Lendo o firmware" />
{:else}
  <section class="secao">
    <h2>Ficha do firmware <span class="selo">só leitura</span></h2>
    <p class="apoio">O que o Windows deixa ler da BIOS e do hardware. O Otimiza não grava na BIOS: em placa de consumo, um erro ali deixa a placa sem ligar.</p>
    {#if erro}<p class="erro">{erro}</p>{/if}
    <dl class="ficha">
      {#each linhas as [k, v] (k)}<dt>{k}</dt><dd>{v}</dd>{/each}
    </dl>
    {#if lacunas.length}<p class="apoio pequeno">{lacunas.join(" ")}</p>{/if}
    <div class="acoes">
      <button class="btn" onclick={copiar}>Copiar a ficha</button>
      <button class="btn" onclick={salvar}>Salvar em arquivo</button>
    </div>
    {#if recado}<p class="apoio" role="status">{recado}</p>{/if}
  </section>

  <section class="secao">
    <h2>Defeitos conhecidos</h2>
    {#if ficha?.defeitos.length}
      {#each ficha.defeitos as d, i (i)}<p class="aviso">{fraseDoDefeito(d)}</p>{/each}
    {:else if conferido}
      <p class="apoio">Nenhum defeito conhecido para este processador e esta leitura.</p>
    {:else}
      <p class="apoio">Não deu para ler o processador, e sem ele os defeitos conhecidos não foram conferidos.</p>
    {/if}
  </section>

  {#if firmware?.passos.length}
    <section class="secao">
      <h2>O que olhar na BIOS, em ordem de risco</h2>
      <ol class="passos">
        {#each firmware.passos as p, i (i)}
          <li>
            <span class="titulo">{p.titulo}{#if p.medido_aqui}&nbsp;<span class="selo medido">medido aqui</span>{/if}</span>
            <span class="apoio"><b>Onde:</b> {p.onde}</span>
            <span class="apoio"><b>O que faz:</b> {p.o_que_faz}</span>
            <span class="apoio"><b>Risco e volta:</b> {p.risco_e_volta}</span>
          </li>
        {/each}
      </ol>
    </section>
  {/if}

  <section class="secao">
    <h2>Antes e depois da BIOS</h2>
    {#if ficha?.notebook}
      <p class="apoio">Em notebook o fabricante costuma travar a maior parte do menu da BIOS (perfil de memória, Resizable BAR, limites de energia). Se a opção não aparecer, ela não existe nesta máquina.</p>
    {/if}
    {#if !comparacao}
      <p class="apoio">Antes de mexer na BIOS, guarde como está agora. Na volta, aqui aparece o que mudou.</p>
    {:else if comparacao.mudou.length === 0}
      <p class="apoio">Nada mudou desde a foto de {comparacao.quando} no que foi lido dos dois lados.{comparacao.fora ? ` ${comparacao.fora} item(ns) não foram lidos em um dos lados e ficaram fora.` : ""}</p>
    {:else}
      <p class="destaque">Desde a foto de {comparacao.quando}:</p>
      <dl class="ficha">
        {#each comparacao.mudou as m (m.k)}<dt>{m.k}</dt><dd>{m.antes} → <b>{m.depois}</b></dd>{/each}
      </dl>
    {/if}
    <div class="acoes">
      <button class="btn" onclick={() => (recado = guardarFoto() ? "Guardado. Na volta da BIOS, aqui aparece o que mudou." : "Não deu para guardar neste computador.")}>Guardar como está agora</button>
      <button class="btn primario" disabled={reiniciando || firmware?.leitura.uefi === false} onclick={reiniciarNaBios}>Reiniciar na BIOS</button>
    </div>
    <p class="apoio pequeno">
      {firmware?.leitura.uefi === false
        ? "Em modo Legacy o Windows não reinicia direto na BIOS: reinicie e aperte Del ou F2 enquanto a marca da placa aparece."
        : "Reiniciar na BIOS guarda a foto antes e fecha o Windows em 5 segundos. Salve o que estiver aberto."}
    </p>
  </section>
{/if}

<style>
  .ficha {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 24px;
    margin: 4px 0;
    font-size: 13px;
  }
  .ficha dt {
    color: var(--tinta-2);
  }
  .ficha dd {
    margin: 0;
  }
  .acoes {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .aviso {
    color: var(--atencao);
    font-size: 13px;
  }
  .erro {
    color: var(--perigo);
  }
  .destaque {
    font-weight: 500;
  }
  .passos {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 12px;
  }
  .passos li {
    display: grid;
    gap: 2px;
  }
  .titulo {
    font-weight: 500;
  }
  .pequeno {
    font-size: 12px;
  }
</style>
