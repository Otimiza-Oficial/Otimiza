// A página da janela da mira. A configuração chega no endereço (`mira.html#<base64>`) e as trocas pelo
// `window.mira(...)` que o backend chama. Tudo que chega já foi validado em `plataforma::mira::validar`.

import { desenharMira, PADRAO, type Mira } from "./mira";

function desenhar(m: Mira) {
  const svg = document.getElementById("mira");
  if (svg) svg.innerHTML = desenharMira({ ...PADRAO, ...m });
}

function doEndereco(): Mira {
  try {
    const b64 = location.hash.slice(1).replace(/-/g, "+").replace(/_/g, "/");
    const json = new TextDecoder().decode(Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)));
    return { ...PADRAO, ...(JSON.parse(json) as Partial<Mira>) };
  } catch {
    return { ...PADRAO };
  }
}

(window as unknown as { mira: (m: Mira) => void }).mira = desenhar;
desenhar(doEndereco());
