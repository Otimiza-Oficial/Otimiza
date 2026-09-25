import { invoke } from "@tauri-apps/api/core";

/*
 * Aba BIOS (3.0). Não grava na BIOS (NVRAM de cada fabricante; errar deixa a placa sem ligar): ficha do que o
 * Windows deixa ler, defeitos conhecidos, atalho para reiniciar na BIOS e, na volta, o que mudou contra uma foto
 * guardada neste computador (localStorage). Sem foto, sem comparação, e a tela diz.
 */

type Defeito =
  | { tipo: "MicrocodigoIntelAntigo"; atual: number }
  | { tipo: "BootLentoAm5"; segundos: number }
  | { tipo: "BootLento"; segundos: number };

type Ficha = {
  leitura: {
    cpu: string | null;
    microcodigo: number | null;
    integridade_de_memoria: boolean | null;
    tempo_da_bios_s: number | null;
    lacunas: string[];
  };
  defeitos: Defeito[];
  notebook: boolean;
};

type Passo = { fase: string; titulo: string; onde: string; o_que_faz: string; risco_e_volta: string; medido_aqui: boolean };

type Firmware = {
  leitura: {
    placa_mae: string | null;
    versao_da_bios: string | null;
    data_da_bios: string | null;
    uefi: boolean | null;
    secure_boot: boolean | null;
    lacunas: string[];
  };
  passos: Passo[];
};

type Memoria = { slots: number | null; pentes_gb: number[]; canais: number; mhz: number | null };

type Foto = { quando: number; valores: Record<string, string> };

const CHAVE_DA_FOTO = "otimiza.bios.foto";

let raiz: HTMLElement | null = null;
let carregado = false;
let valoresAgora: Record<string, string> = {};
let textoDaFicha = "";

function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}

const simNao = (v: boolean | null | undefined, sim: string, nao: string) => (v === true ? sim : v === false ? nao : "não deu para ler");

/** "20230512000000.000000+000" → "12/05/2023". */
function dataDaBios(bruta: string | null): string | null {
  const m = bruta?.match(/^(\d{4})(\d{2})(\d{2})/);
  return m ? `${m[3]}/${m[2]}/${m[1]}` : bruta;
}

function lerFoto(): Foto | null {
  try {
    const t = localStorage.getItem(CHAVE_DA_FOTO);
    return t ? (JSON.parse(t) as Foto) : null;
  } catch {
    return null;
  }
}

function guardarFoto(): boolean {
  try {
    localStorage.setItem(CHAVE_DA_FOTO, JSON.stringify({ quando: Date.now(), valores: valoresAgora } satisfies Foto));
    return true;
  } catch {
    return false;
  }
}

function fraseDoDefeito(d: Defeito): string {
  switch (d.tipo) {
    case "MicrocodigoIntelAntigo":
      return `<strong>O microcódigo do processador é o 0x${d.atual.toString(16).toUpperCase()}.</strong> A Intel publicou o 0x12F como correção da instabilidade que degrada com o tempo os processadores de mesa da 13ª e 14ª geração com o chip Raptor Lake, como este. Ele chega por atualização de BIOS: procure na página oficial da sua placa uma versão que cite o microcódigo 0x12F. O Otimiza não atualiza BIOS.`;
    case "BootLentoAm5":
      return `<strong>A placa-mãe levou ${d.segundos} s no último boot.</strong> Em placas AM5 com o perfil EXPO ligado, o mais comum é a placa treinar a memória a cada boot. A opção <em>Memory Context Restore</em> reaproveita o treino e costuma cortar esse tempo. Se o PC ficar instável depois de ligar, desligue de novo.`;
    case "BootLento":
      return `<strong>A placa-mãe levou ${d.segundos} s no último boot.</strong> Vale conferir na BIOS se o <em>Fast Boot</em> está ligado e se o modo CSM (Legacy) está desligado.`;
  }
}

const semHtml = (s: string) => s.replace(/<[^>]+>/g, "");

/** O texto de copiar e de salvar: a ficha, os defeitos e o passo a passo, como a tela mostra. */
function textoCompleto(linhas: [string, string][], f: Ficha | null, fw: Firmware | null): string {
  const partes = ["Ficha do firmware (Otimiza)", ...linhas.map(([k, v]) => `${k}: ${v}`)];
  const defeitos = f?.defeitos ?? [];
  partes.push("", "Defeitos conhecidos:", ...(defeitos.length ? defeitos.map((d) => "- " + semHtml(fraseDoDefeito(d))) : [f?.leitura.cpu ? "- nenhum" : "- não conferido"]));
  const passos = fw?.passos ?? [];
  if (passos.length) {
    partes.push("", "O que olhar na BIOS, em ordem de risco:");
    for (const p of passos) {
      partes.push(`- ${p.titulo}${p.medido_aqui ? " (medido aqui)" : ""}`, `  Onde: ${p.onde}`, `  O que faz: ${p.o_que_faz}`, `  Risco e volta: ${p.risco_e_volta}`);
    }
  }
  const lacunas = [...(fw?.leitura.lacunas ?? []), ...(f?.leitura.lacunas ?? [])];
  if (lacunas.length) partes.push("", "Não deu para ler:", ...lacunas.map((l) => "- " + l));
  return partes.join("\n");
}

function montarValores(f: Ficha | null, fw: Firmware | null, m: Memoria | null): [string, string][] {
  const l = fw?.leitura;
  const c = f?.leitura;
  const linhas: [string, string][] = [
    ["Placa-mãe", l?.placa_mae ?? "não deu para ler"],
    ["Versão da BIOS", l?.versao_da_bios ? `${l.versao_da_bios}${l.data_da_bios ? ` · ${dataDaBios(l.data_da_bios)}` : ""}` : "não deu para ler"],
    ["Modo de inicialização", l ? simNao(l.uefi, "UEFI", "Legacy (CSM)") : "não deu para ler"],
    ["Secure Boot", l ? simNao(l.secure_boot, "ligado", "desligado") : "não deu para ler"],
    ["Processador", c?.cpu ?? "não deu para ler"],
  ];
  if (c?.microcodigo != null) linhas.push(["Microcódigo", `0x${c.microcodigo.toString(16).toUpperCase()}`]);
  linhas.push(["Integridade de Memória", c ? simNao(c.integridade_de_memoria, "ligada", "desligada") : "não deu para ler"]);
  linhas.push([
    "Memória",
    m ? `${m.pentes_gb.reduce((a, b) => a + b, 0)} GB · ${m.mhz ? `${m.mhz} MHz` : "velocidade não lida"} · ${m.canais === 1 ? "canal único" : `${m.canais} canais`}` : "não deu para ler",
  ]);
  linhas.push(["Tempo da placa no boot", c?.tempo_da_bios_s != null ? `${c.tempo_da_bios_s.toFixed(1).replace(".", ",")} s` : "não registrado"]);
  return linhas;
}

function blocoDaComparacao(): string {
  const foto = lerFoto();
  if (!foto) {
    return `<p class="fg-nota">Antes de mexer na BIOS, guarde como está agora. Na volta, esta aba mostra o que mudou.</p>`;
  }
  // O que não se leu em um dos lados não entra: "não deu para ler" igual dos
  // dois lados não é "nada mudou".
  const lido = (v: string | undefined) => v !== undefined && v !== "não deu para ler" && v !== "não registrado";
  const comparaveis = Object.entries(valoresAgora).filter(([k, v]) => lido(foto.valores[k]) && lido(v));
  const mudou = comparaveis.filter(([k, v]) => foto.valores[k] !== v);
  const quando = new Date(foto.quando).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" });
  const semLeitura = Object.keys(valoresAgora).length - comparaveis.length;
  const nota = semLeitura ? ` ${semLeitura} item(ns) não foram lidos em um dos lados e ficaram fora da comparação.` : "";
  if (mudou.length === 0) {
    return `<p class="fg-aviso">Nada mudou desde a foto de ${esc(quando)} no que foi lido dos dois lados.${nota}</p>`;
  }
  return `<p class="fg-aviso"><strong>Desde a foto de ${esc(quando)}:</strong></p>
    <dl class="fg-grade">${mudou
      .map(([k, v]) => `<div><dt>${esc(k)}</dt><dd>${esc(foto.valores[k])} → <strong>${esc(v)}</strong></dd></div>`)
      .join("")}</dl>${nota ? `<p class="fg-nota">${nota.trim()}</p>` : ""}`;
}

function desenhar(f: Ficha | null, fw: Firmware | null, m: Memoria | null, erro: string | null) {
  if (!raiz) return;
  const linhas = montarValores(f, fw, m);
  valoresAgora = Object.fromEntries(linhas);
  textoDaFicha = textoCompleto(linhas, f, fw);

  const defeitos = f?.defeitos ?? [];
  // Sem o processador lido, "nenhum defeito" seria afirmar o que não se conferiu.
  const conferido = !!f?.leitura.cpu;
  const lacunas = [...(fw?.leitura.lacunas ?? []), ...(f?.leitura.lacunas ?? [])];
  const uefi = fw?.leitura.uefi;

  raiz.innerHTML = `
    <section class="panel fg-painel">
      <div class="panel-head"><h2>Ficha do firmware</h2><span class="panel-tag">só leitura</span></div>
      <p class="fg-lead">O que o Windows deixa ler da BIOS e do hardware. O Otimiza não grava na BIOS: em placa de consumo, um erro ali deixa a placa sem ligar.</p>
      ${erro ? `<p class="fg-erro">${esc(erro)}</p>` : ""}
      <dl class="fg-grade">${linhas.map(([k, v]) => `<div><dt>${esc(k)}</dt><dd>${esc(v)}</dd></div>`).join("")}</dl>
      ${lacunas.length ? `<p class="fg-nota">${lacunas.map(esc).join(" ")}</p>` : ""}
      <div class="fg-linha">
        <button class="btn" type="button" data-bios="copiar">Copiar a ficha</button>
        <button class="btn" type="button" data-bios="salvar">Salvar em arquivo</button>
        <span class="fg-nota" id="bios-copiada" role="status"></span>
      </div>
    </section>

    <section class="panel fg-painel">
      <div class="panel-head"><h2>Defeitos conhecidos</h2><span class="panel-tag">${defeitos.length ? `${defeitos.length} nesta máquina` : conferido ? "nenhum" : "não conferido"}</span></div>
      ${defeitos.length
        ? defeitos.map((d) => `<p class="fg-aviso">${fraseDoDefeito(d)}</p>`).join("")
        : conferido
          ? `<p class="fg-nota">Nenhum defeito conhecido para este processador e esta leitura.</p>`
          : `<p class="fg-nota">Não deu para ler o processador, e sem ele os defeitos conhecidos não foram conferidos.</p>`}
    </section>

    <section class="panel fg-painel">
      <div class="panel-head"><h2>Antes e depois da BIOS</h2></div>
      ${f?.notebook ? `<p class="fg-nota">Em notebook o fabricante costuma travar a maior parte do menu da BIOS (perfil de memória, Resizable BAR, limites de energia). Se a opção não aparecer, ela não existe nesta máquina.</p>` : ""}
      ${blocoDaComparacao()}
      <div class="fg-linha">
        <button class="btn" type="button" data-bios="foto">Guardar como está agora</button>
        <button class="btn btn-primary" type="button" data-bios="entrar" ${uefi === false ? "disabled" : ""}>Reiniciar na BIOS</button>
      </div>
      <p class="fg-nota" id="bios-status" role="status">${uefi === false
        ? "Em modo Legacy o Windows não reinicia direto na BIOS: reinicie e aperte Del ou F2 enquanto a marca da placa aparece."
        : "Reiniciar na BIOS guarda a foto antes e fecha o Windows em 5 segundos. Salve o que estiver aberto."}</p>
    </section>`;
}

async function carregar() {
  const [f, fw, m] = await Promise.allSettled([
    invoke<Ficha>("ficha_da_bios"),
    invoke<Firmware>("passo_a_passo_da_bios"),
    invoke<Memoria>("memoria_instalada"),
  ]);
  const erro = [f, fw].find((r) => r.status === "rejected") as PromiseRejectedResult | undefined;
  desenhar(
    f.status === "fulfilled" ? f.value : null,
    fw.status === "fulfilled" ? fw.value : null,
    m.status === "fulfilled" ? m.value : null,
    erro ? String(erro.reason) : null,
  );
}

function ligarEventos() {
  raiz?.addEventListener("click", async (e) => {
    const botao = (e.target as HTMLElement).closest<HTMLButtonElement>("button[data-bios]");
    if (!botao || !raiz) return;
    const status = raiz.querySelector<HTMLElement>("#bios-status");
    const acao = botao.dataset.bios;

    if (acao === "copiar") {
      const aviso = raiz.querySelector<HTMLElement>("#bios-copiada");
      try {
        await navigator.clipboard.writeText(textoDaFicha);
        if (aviso) aviso.textContent = "Copiada. Cole no Discord ou mande ao suporte.";
      } catch {
        if (aviso) aviso.textContent = "Não deu para copiar.";
      }
    }

    if (acao === "salvar") {
      const aviso = raiz.querySelector<HTMLElement>("#bios-copiada");
      try {
        const caminho = await invoke<string>("salvar_ficha_da_bios", { texto: textoDaFicha });
        if (aviso) aviso.textContent = `Salva em ${caminho}`;
      } catch (erro) {
        if (aviso) aviso.textContent = String(erro);
      }
    }

    if (acao === "foto") {
      if (status) status.textContent = guardarFoto() ? "Guardado. Na volta da BIOS, esta aba mostra o que mudou." : "Não deu para guardar neste computador.";
    }

    if (acao === "entrar") {
      const ok = window.confirm("O PC vai reiniciar direto na tela da BIOS em 5 segundos. Salve o que estiver aberto. Continuar?");
      if (!ok) return;
      guardarFoto();
      botao.disabled = true;
      try {
        await invoke("reiniciar_na_bios");
        if (status) status.textContent = "Reiniciando na BIOS…";
      } catch (erro) {
        botao.disabled = false;
        if (status) status.textContent = String(erro);
      }
    }
  });
}

export async function carregarAbaBios() {
  if (carregado) return;
  carregado = true;
  raiz = document.getElementById("bios-aba");
  if (!raiz) return;
  raiz.innerHTML = `<p class="fg-nota">Lendo o firmware…</p>`;
  ligarEventos();
  await carregar();
}
