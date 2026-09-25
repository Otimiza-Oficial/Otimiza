"use client";

import {
  ChevronLeft,
  CircuitBoard,
  Cpu,
  Gamepad2,
  Layers,
  LayoutGrid,
  Microchip,
  Plug,
  Search,
  SlidersHorizontal,
  Sparkles,
  Zap,
  type LucideIcon,
} from "lucide-react";
import { animate, AnimatePresence, motion, useMotionValue, useTransform } from "framer-motion";
import { useEffect, useState, type ReactNode } from "react";
import { cn } from "@/lib/cn";
import s from "./AppJanela.module.css";

/*
 * O Otimiza desenhado em código, no visual do programa. Os números são de uma máquina de teste real,
 * os mesmos das capturas que esta vitrine substituiu; não invente outros.
 */

export type TelaId = "painel" | "otimizacoes" | "jogos" | "bios";

const LATERAL: { grupo?: string; id?: TelaId; nome: string; icone: LucideIcon }[] = [
  { grupo: "MONITORAR", id: "painel", nome: "Início", icone: LayoutGrid },
  { grupo: "AGIR", id: "otimizacoes", nome: "Otimizações", icone: Zap },
  { nome: "Núcleos", icone: Cpu },
  { nome: "Placa de vídeo", icone: CircuitBoard },
  { id: "jogos", nome: "Jogos", icone: Gamepad2 },
  { nome: "Geração de quadros", icone: Layers },
  { nome: "Energia", icone: Plug },
  { id: "bios", nome: "BIOS", icone: Microchip },
  { nome: "Sistema", icone: SlidersHorizontal },
];

const saida = [0.22, 1, 0.36, 1] as const;

function Contador({ ate, casas = 0, parado }: { ate: number; casas?: number; parado: boolean }) {
  const v = useMotionValue(parado ? ate : 0);
  const texto = useTransform(v, (n) => n.toFixed(casas));
  useEffect(() => {
    if (parado) {
      v.set(ate);
      return;
    }
    const a = animate(v, ate, { duration: 1.1, ease: saida });
    return () => a.stop();
  }, [ate, parado, v]);
  return <motion.span>{texto}</motion.span>;
}

function Barra({ pct, parado, cor }: { pct: number; parado: boolean; cor?: string }) {
  return (
    <div className={s.barra}>
      <motion.span
        initial={{ width: parado ? `${pct}%` : 0 }}
        animate={{ width: `${pct}%` }}
        transition={{ duration: parado ? 0 : 1, ease: saida, delay: parado ? 0 : 0.15 }}
        style={cor ? { background: cor } : undefined}
      />
    </div>
  );
}

/** Pontos numa esfera (espiral de Fibonacci), fixos: o servidor e o navegador desenham igual. */
const PONTOS = Array.from({ length: 220 }, (_, i) => {
  const y = 1 - (i / 219) * 2;
  const r = Math.sqrt(1 - y * y);
  const t = i * 2.399963;
  return { x: 50 + Math.cos(t) * r * 46, y: 50 + y * 46, z: Math.sin(t) * r };
});

function Esfera({ parado }: { parado: boolean }) {
  return (
    <motion.svg
      viewBox="0 0 100 100"
      animate={parado ? undefined : { rotate: 360 }}
      transition={{ duration: 40, ease: "linear", repeat: Infinity }}
    >
      {PONTOS.map((p, i) => (
        <circle key={i} cx={p.x.toFixed(2)} cy={p.y.toFixed(2)} r={0.55} fill="#ff5a6e" opacity={(0.35 + (p.z + 1) * 0.3).toFixed(2)} />
      ))}
    </motion.svg>
  );
}

function Cabeca({ nome, icone: Icone }: { nome: string; icone: LucideIcon }) {
  return (
    <>
      <div className={s.migalha}>OTIMIZA / {nome.toUpperCase()}</div>
      <div className={s.cabeca}>
        <span>
          <Icone strokeWidth={1.75} />
        </span>
        <span>{nome}</span>
      </div>
    </>
  );
}

function Cartao({ titulo, canto, children, className }: { titulo: string; canto?: string; children: ReactNode; className?: string }) {
  return (
    <div className={cn(s.cartao, className)}>
      <div className={s.cartaoTopo}>
        {titulo}
        {canto && <small>{canto}</small>}
      </div>
      <div className={s.cartaoCorpo}>{children}</div>
    </div>
  );
}

function Inicio({ parado }: { parado: boolean }) {
  const pesando = [
    ["msedgewebview2.exe ×6", 17.0],
    ["steamwebhelper.exe ×6", 15.2],
    ["steam.exe", 10.4],
  ] as const;
  return (
    <>
      <Cabeca nome="Início" icone={LayoutGrid} />
      <div className={cn(s.cartao, s.veredito)}>
        <div className={s.esfera}>
          <Esfera parado={parado} />
        </div>
        <div className={s.cartaoCorpo} style={{ paddingLeft: 0 }}>
          <div className={cn(s.rotulo, s.vermelho)}>O QUE ESTÁ TRAVANDO ESTE PC</div>
          <div className={s.forte} style={{ fontSize: "0.95em", margin: "0.5em 0 0.4em" }}>
            Programas pedindo mais memória do que existe
          </div>
          <div className={s.texto}>
            <Contador ate={10.8} casas={1} parado={parado} /> GB prometidos para 7.9 GB de memória física.
          </div>
          <div className={s.texto} style={{ marginTop: "0.6em" }}>
            O PC está se sustentando com disco no lugar de memória, e disco é ordens de grandeza mais lento.
          </div>
          <div className={s.rotulo} style={{ marginTop: "1em" }}>
            TAMBÉM NESTA MÁQUINA
          </div>
          <motion.div
            className={s.linha}
            initial={parado ? false : { opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.9, duration: 0.5, ease: saida }}
          >
            <i className={s.ponto} style={{ background: "#f0b93b" }} />
            <span className={s.texto}>Memória em canal único — 1 pente de 8 GB em 4 slots da placa.</span>
          </motion.div>
        </div>
      </div>
      <div className={s.fileira}>
        <Cartao titulo="USO AGORA" canto="AO VIVO">
          <div style={{ display: "flex", gap: "1.4em", alignItems: "center" }}>
            <div className={s.anel}>
              <svg viewBox="0 0 100 100">
                <circle cx="50" cy="50" r="44" fill="none" stroke="rgb(255 255 255 / 0.1)" strokeWidth="4" />
                <motion.circle
                  cx="50"
                  cy="50"
                  r="44"
                  fill="none"
                  stroke="#f0b93b"
                  strokeWidth="4"
                  initial={{ pathLength: parado ? 0.67 : 0 }}
                  animate={{ pathLength: 0.67 }}
                  transition={{ duration: parado ? 0 : 1.1, ease: saida }}
                />
              </svg>
              <div className={s.anelMeio}>
                <strong>
                  <Contador ate={67} parado={parado} />
                </strong>
                <div className={s.rotulo}>% CPU</div>
              </div>
            </div>
            <div style={{ flex: 1 }}>
              <div className={s.rotulo}>MEMÓRIA · 90%</div>
              <Barra pct={90} parado={parado} />
              <div className={s.rotulo} style={{ marginTop: "1em" }}>
                DISCO · 32%
              </div>
              <Barra pct={32} parado={parado} />
            </div>
          </div>
        </Cartao>
        <Cartao titulo="QUEM ESTÁ PESANDO AGORA" canto="AO VIVO">
          {pesando.map(([nome, pct]) => (
            <div key={nome} style={{ marginBottom: "0.7em" }}>
              <div style={{ display: "flex", justifyContent: "space-between" }} className={s.forte}>
                <span style={{ fontSize: "0.9em" }}>{nome}</span>
                <span style={{ fontSize: "0.9em" }}>{pct.toFixed(1)}%</span>
              </div>
              <Barra pct={pct * 5} parado={parado} />
            </div>
          ))}
        </Cartao>
      </div>
    </>
  );
}

function Otimizacoes({ parado }: { parado: boolean }) {
  const [aplicado, setAplicado] = useState(parado);
  useEffect(() => {
    if (parado) return;
    const t = setTimeout(() => setAplicado(true), 1600);
    return () => clearTimeout(t);
  }, [parado]);
  const itens: [string, boolean][] = [
    ["Plano de energia Alto Desempenho", aplicado],
    ["Liberar limites de inicialização", true],
    ["Prioridade para o programa em primeiro plano", false],
    ["Desativar hibernação", true],
  ];
  return (
    <>
      <Cabeca nome="Otimizações" icone={Zap} />
      <div className={s.fileira} style={{ marginTop: 0 }}>
        <Cartao titulo="APLICAR" canto="14 A APLICAR">
          <div className={s.texto}>
            Das 14 a aplicar, 2 mudam o FPS de forma mensurável. Outras 7 liberam recursos de fundo e não mudam FPS — valem
            pela limpeza, não pelo jogo.
          </div>
          <div className={s.botoes}>
            <span className={cn(s.botao, s.botaoForte)}>OTIMIZAR AGORA</span>
            <span className={s.botao}>DESFAZER TUDO</span>
          </div>
          <div className={s.texto} style={{ marginTop: "0.9em", fontSize: "0.58em" }}>
            Medir antes, otimizar, medir de novo. Se o ganho não aparecer nos números, o painel diz.
          </div>
        </Cartao>
        <Cartao titulo="CATÁLOGO" canto="POR CATEGORIA">
          {itens.map(([nome, feito]) => (
            <div key={nome} className={s.linha}>
              <i className={s.ponto} style={{ background: feito ? "#6ee7a0" : "#f0b93b" }} />
              <span className={s.forte} style={{ fontWeight: 500, fontSize: "0.64em" }}>
                {nome}
              </span>
              <AnimatePresence mode="wait" initial={false}>
                <motion.span
                  key={String(feito)}
                  className={cn(s.acao, !feito && s.acaoBotao, feito && s.verde)}
                  initial={{ opacity: 0, y: 4 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -4 }}
                  transition={{ duration: 0.25 }}
                >
                  {feito ? "JÁ OTIMIZADO" : "APLICAR"}
                </motion.span>
              </AnimatePresence>
            </div>
          ))}
        </Cartao>
      </div>
    </>
  );
}

function Ventoinha({ parado }: { parado: boolean }) {
  return (
    <motion.svg
      className={s.ventoinha}
      viewBox="0 0 40 40"
      animate={parado ? undefined : { rotate: 360 }}
      transition={{ duration: 1.6, ease: "linear", repeat: Infinity }}
    >
      <circle cx="20" cy="20" r="18.5" fill="#0c0c0c" stroke="rgb(255 255 255 / 0.18)" />
      {[0, 72, 144, 216, 288].map((a) => (
        <path key={a} d="M20 20 C 22 12, 28 8, 31 9 C 28 14, 25 18, 20 20" fill="#2a2a2a" transform={`rotate(${a} 20 20)`} />
      ))}
      <circle cx="20" cy="20" r="4" fill="currentColor" />
    </motion.svg>
  );
}

function Jogos({ parado }: { parado: boolean }) {
  return (
    <>
      <Cabeca nome="Jogos" icone={Gamepad2} />
      <div className={s.cartao}>
        <div className={s.cartaoCorpo} style={{ display: "flex", gap: "1.8em", alignItems: "center" }}>
          <div className={s.placa}>
            <Ventoinha parado={parado} />
            <Ventoinha parado={parado} />
          </div>
          <div style={{ flex: 1 }}>
            <div className={s.rotulo} style={{ color: "#76b900" }}>
              NVIDIA
            </div>
            <div className={s.forte} style={{ fontSize: "0.95em", margin: "0.3em 0 0.8em" }}>
              NVIDIA GeForce GTX 1650
            </div>
            <div style={{ display: "flex", gap: "2.4em" }}>
              {[
                ["DRIVER", "32.0.16.1664"],
                ["MEMÓRIA DE VÍDEO", "4 GB"],
              ].map(([r, v]) => (
                <div key={r}>
                  <div className={s.rotulo}>{r}</div>
                  <div className={s.forte}>{v}</div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
      <Cartao titulo="CONFIGURAÇÃO DO JOGO" className={s.fileiraTopo}>
        <div className={s.texto}>
          Esta é a parte que mais muda o seu FPS, e a que quase nenhum otimizador toca. Numa placa de entrada, a suavização de
          serrilhado sozinha custa entre 30% e 50% dos quadros.
        </div>
        <div className={s.texto} style={{ marginTop: "0.6em", paddingLeft: "0.8em", borderLeft: "2px solid rgb(255 255 255 / 0.15)" }}>
          O arquivo inteiro é guardado antes de qualquer mudança, e o desfazer devolve ele byte a byte.
        </div>
        <div className={s.botoes}>
          <span className={s.botao}>ANALISAR</span>
          <span className={cn(s.botao, s.botaoForte)}>TIRAR O LIMITE DE FPS</span>
          <span className={s.botao}>EQUILIBRADO</span>
          <span className={s.botao}>COMPETITIVO</span>
        </div>
      </Cartao>
    </>
  );
}

function Bios() {
  return (
    <>
      <Cabeca nome="BIOS" icone={Microchip} />
      <div className={s.fileira} style={{ marginTop: 0 }}>
        <Cartao titulo="FICHA DO FIRMWARE" canto="SÓ LEITURA">
          {[
            ["MEMÓRIA", "8 GB · 2667 MHz"],
            ["ENCAIXES", "1 de 4 · canal único"],
          ].map(([r, v]) => (
            <div key={r} className={s.linha}>
              <span className={s.rotulo}>{r}</span>
              <span className={s.forte} style={{ marginLeft: "auto" }}>
                {v}
              </span>
            </div>
          ))}
          <div className={s.texto} style={{ marginTop: "0.8em", paddingLeft: "0.8em", borderLeft: "2px solid #f0b93b" }}>
            O Otimiza não grava na BIOS: em placa de consumo, um erro ali deixa a placa sem ligar.
          </div>
          <div className={s.botoes}>
            <span className={cn(s.botao, s.botaoForte)}>REINICIAR NA BIOS</span>
            <span className={s.botao}>SALVAR EM ARQUIVO</span>
          </div>
        </Cartao>
        <Cartao titulo="O QUE OLHAR NO MENU" canto="EM ORDEM DE RISCO">
          {[
            ["Perfil da memória (XMP / EXPO)", "medido: 2667 MHz", "#f0b93b"],
            ["Resizable BAR", "ler no menu", "#8a8a88"],
            ["Atualizar a BIOS", "só pelo site da placa", "#ff5a6e"],
          ].map(([nome, nota, cor], i) => (
            <motion.div
              key={nome}
              className={s.linha}
              initial={{ opacity: 0, x: -6 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: 0.15 + i * 0.12, duration: 0.4, ease: saida }}
            >
              <i className={s.ponto} style={{ background: cor }} />
              <span className={s.forte} style={{ fontWeight: 500, fontSize: "0.64em" }}>
                {nome}
              </span>
              <span className={s.acao}>{nota.toUpperCase()}</span>
            </motion.div>
          ))}
          <div className={s.texto} style={{ marginTop: "0.6em", fontSize: "0.58em" }}>
            Na volta, o Otimiza compara com a foto que tirou antes e mostra o que mudou.
          </div>
        </Cartao>
      </div>
    </>
  );
}

function Conteudo({ tela, parado }: { tela: TelaId; parado: boolean }) {
  switch (tela) {
    case "painel":
      return <Inicio parado={parado} />;
    case "otimizacoes":
      return <Otimizacoes parado={parado} />;
    case "jogos":
      return <Jogos parado={parado} />;
    case "bios":
      return <Bios />;
  }
}

/** `aberta`: a janela já entrou na tela; antes disso ela espera, para a abertura acontecer à vista. */
export function AppJanela({ tela, aberta, parado }: { tela: TelaId; aberta: boolean; parado: boolean }) {
  const mostrar = parado || aberta;
  return (
    <div className={s.caixa} aria-hidden="true">
      <motion.div
        className={s.janela}
        initial={parado ? false : { opacity: 0, scale: 0.94, y: 24 }}
        animate={mostrar ? { opacity: 1, scale: 1, y: 0 } : { opacity: 0, scale: 0.94, y: 24 }}
        transition={parado ? { duration: 0 } : { duration: 0.8, ease: saida }}
      >
        <div className={s.titulo}>
          <i className={s.bolinha} style={{ background: "#ff5f57" }} />
          <i className={s.bolinha} style={{ background: "#febc2e" }} />
          <i className={s.bolinha} style={{ background: "#28c840" }} />
          <span className={s.tituloTexto}>OTIMIZA</span>
        </div>

        <div className={s.topo}>
          <div className={s.busca}>
            <Search size="1.1em" />
            Buscar ação…
            <span className={s.tecla}>Ctrl K</span>
          </div>
          <div className={s.medidores}>
            {[
              ["CPU", 67, "#f2f2f0"],
              ["MEMÓRIA", 90, "#ff5a6e"],
              ["DISCO", 32, "#f2f2f0"],
            ].map(([nome, pct, cor]) => (
              <div key={nome} className={s.medidor}>
                {nome}
                <b style={nome === "MEMÓRIA" ? { color: "#ff5a6e" } : undefined}>{pct}%</b>
                <div className={s.trilho}>
                  <motion.span
                    initial={{ width: parado ? `${pct}%` : 0 }}
                    animate={mostrar ? { width: `${pct}%` } : { width: 0 }}
                    transition={parado ? { duration: 0 } : { duration: 1.2, ease: saida, delay: 0.4 }}
                    style={{ background: cor as string }}
                  />
                </div>
              </div>
            ))}
          </div>
          <span className={s.admin}>● ADMINISTRADOR</span>
        </div>

        <div className={s.aviso}>
          <i />
          Programas pedindo mais memória do que existe
          <small>VER NO PAINEL</small>
        </div>

        <div className={s.corpo}>
          <nav className={s.lateral}>
            <div className={s.marca}>
              <Sparkles size="1em" />
              Otimiza
              <ChevronLeft size="1em" style={{ marginLeft: "auto", opacity: 0.6 }} />
            </div>
            {LATERAL.map((item, i) => {
              const ativo = item.id === tela;
              const Icone = item.icone;
              return (
                <div key={item.nome}>
                  {item.grupo && <div className={s.grupo}>{item.grupo}</div>}
                  <motion.div
                    className={cn(s.item, ativo && s.itemAtivo)}
                    initial={parado ? false : { opacity: 0, x: -10 }}
                    animate={mostrar ? { opacity: 1, x: 0 } : { opacity: 0, x: -10 }}
                    transition={parado ? { duration: 0 } : { delay: 0.35 + i * 0.04, duration: 0.4, ease: saida }}
                  >
                    {ativo && (
                      <motion.i
                        layoutId={parado ? undefined : "otimiza-realce"}
                        className={s.realce}
                        transition={{ type: "spring", stiffness: 420, damping: 36 }}
                      />
                    )}
                    <Icone strokeWidth={1.75} />
                    <span>{item.nome}</span>
                  </motion.div>
                </div>
              );
            })}
          </nav>

          <div className={s.conteudo}>
            <AnimatePresence mode="wait" initial={false}>
              <motion.div
                key={tela}
                initial={parado ? false : { opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={parado ? undefined : { opacity: 0, y: -6 }}
                transition={{ duration: 0.35, ease: saida }}
              >
                {mostrar && <Conteudo tela={tela} parado={parado} />}
              </motion.div>
            </AnimatePresence>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
