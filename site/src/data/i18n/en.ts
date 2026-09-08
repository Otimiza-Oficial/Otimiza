/**
 * ENGLISH — translated from pt.ts, which is the source of truth.
 *
 * TRANSLATION RULES FOR THIS FILE, IN ORDER OF IMPORTANCE:
 *
 * 1. No number changes. 677 tests, 25% CPU, 40 ms, 8 GB are measurements and
 *    measurements do not get translated. Only the decimal separator changes:
 *    "1,2 s" in Portuguese is "1.2 s" here, same quantity.
 *
 * 2. No upgrading the copy. The Portuguese says "admits when nothing changed";
 *    the English says that too, not "maximize your performance". The whole
 *    product is an accusation that this market advertises numbers nobody can
 *    verify — empty marketing in the English version would destroy the only
 *    argument the product has.
 *
 * 3. Windows terms use their real English names: "registro do Windows" is the
 *    "Windows Registry"; the SmartScreen strings below are the actual ones the
 *    English installer shows.
 *
 * 4. Careful with pilar 02: "agendador do Windows" there is the OS thread
 *    scheduler, NOT Task Scheduler. Translating it as "Task Scheduler" would
 *    make a true sentence false.
 *
 * 5. Price stays in Brazilian reais. Mercado Pago charges in BRL; printing a
 *    dollar figure would be lying about what the checkout takes.
 */
import type { Conteudo } from "./tipos";

export const en: Conteudo = {
  meta: {
    htmlLang: "en",
    hreflang: "en",
    ogLocale: "en_US",
    localeNumero: "en-US",
    sigla: "EN",
    nome: "English",
    titulo: "Otimiza — Performance console for Windows",
    descricao:
      "Measures what your PC does, optimizes what it can, and proves it with numbers — including when the number says nothing changed.",
  },

  a11y: {
    pularConteudo: "Skip to content",
    secoes: "Sections",
    abrirMenu: "Open section menu",
    trocarTema: "Switch between light and dark theme",
    idioma: "Language",
    linksRodape: "Footer links",
    numerosMedidos: "Measured numbers",
    capturaPendente: "Screenshot pending",
  },

  cabecalho: {
    links: [
      { href: "#pilares", texto: "How it works" },
      { href: "#recusas", texto: "What it won't do" },
      { href: "#privacidade", texto: "Privacy" },
    ],
    comprar: "Buy",
    menu: "Menu",
    preco: "Price",
  },

  hero: {
    promessa: "Performance console for Windows",
    chamada:
      "Measures what your PC does, optimizes what it can, and proves it with numbers — including when the number says nothing changed.",
    texto:
      "A progress bar, a list of tweaks copied off the internet, and a made-up gain at the end. " +
      "Otimiza is built on the refusal to do that.",
    ctaBaixar: "Download for Windows",
    ctaComprar: "See the price",
    ctaTelas: "See the screens",
    versaoPrefixo: "Version",
    linhaMeta:
      "Free to download · activation R$ 20 BRL, once · Windows 10 and 11, 64-bit",
    captura: {
      titulo: "Dashboard",
      descricao: "the opening screen: what is holding this PC back, measured on the spot",
    },
  },

  numeros: [
    { valor: "677", legenda: "automated tests, zero warnings", fonte: "1.7.0 commit" },
    {
      valor: "1.2 s",
      legenda: "to launch, down from 3.7 s in the previous version",
      fonte: "1.7.0 release notes",
    },
    { valor: "byte for byte", legenda: "the precision of undo", fonte: "PROGRESS.md" },
    { valor: "zero", legenda: "of your data leaves your machine", fonte: "only the version question goes out, anonymously" },
  ],

  pilares: {
    rotulo: "THE PROBLEM",
    titulo: "Every PC optimizer advertises a number you have no way to check.",
    itens: [
      {
        numero: "01",
        rotulo: "MEASUREMENT",
        titulo: "Measures before and after — and admits when nothing changed",
        texto:
          "The noise thresholds were not guessed: they came from a test that measures the same " +
          "machine three times without changing anything. Whatever varied there is noise, and it is " +
          "never reported as a gain.",
        detalhe:
          "Two of the six metrics produce no verdict at all — calibration proved they fluctuate too much on their own.",
        fonte: "README.md · noise_calibration",
      },
      {
        numero: "02",
        rotulo: "THE STUTTER",
        titulo: "Measures the stutter, not average FPS",
        texto:
          "Nobody complains about a low average — they complain that the game stutters. A 40 ms " +
          "freeze ruins smoothness and barely moves an average of 60 frames per second.",
        /* "agendador do Windows" = the OS thread scheduler, not Task Scheduler. */
        detalhe:
          "Otimiza reads the Windows scheduler delay directly, which is what the player feels.",
        fonte: "README.md · jitter module",
      },
      {
        numero: "03",
        rotulo: "REFUSAL",
        titulo: "Refuses to measure while the PC is busy",
        texto:
          "Comparing a busy PC with a rested one invents a gain of tens of percent. " +
          "Above 25% CPU usage, no verdict is issued.",
        detalhe: "And the screen explains why, instead of showing a number that isn't worth anything.",
        fonte: "README.md",
      },
      {
        numero: "04",
        rotulo: "HARDWARE",
        titulo: "Reads your machine before offering anything",
        texto:
          "Disabling SysMain helps on an SSD and hurts on a mechanical drive. Turning off memory " +
          "compression helps when RAM is plentiful and makes things worse with 8 GB.",
        detalhe: "Otimiza detects the hardware and doesn't offer what would hurt it.",
        fonte: "README.md",
      },
      {
        numero: "05",
        rotulo: "REVERSIBLE",
        titulo: "Undo restores it identical, byte for byte",
        texto:
          "Every change records the previous state before writing. Undo does not restore something " +
          "equivalent: it restores exactly what was there.",
        detalhe:
          "The full cycle was run against the Windows Registry and checked from the outside, with PowerShell.",
        fonte: "PROGRESS.md · real cycle",
      },
      {
        numero: "06",
        rotulo: "ALREADY OPTIMIZED",
        titulo: "Says when there is nothing to do",
        texto:
          "If the setting is already applied, it shows “already optimized” instead of faking work " +
          "and taking credit for a change that never happened.",
        detalhe: null,
        fonte: "README.md",
      },
    ],
    provaRotulo: "THE PROOF",
    provaTitulo: "Before and after, side by side, with the verdict for each metric.",
    provaCaptura: {
      titulo: "Optimizations",
      descricao: "measure first, optimize, measure again — and the catalog saying what is already optimized",
    },
  },

  telas: {
    rotulo: "THE SCREENS",
    titulo: "The whole program, with no hidden screen.",
    itens: [
      {
        arquivo: "diagnostico.png",
        titulo: "Diagnostics",
        descricao: "monitors, installed memory and the findings on this machine",
      },
      {
        arquivo: "jogos.png",
        titulo: "Games",
        descricao: "the graphics card, the driver, and the game setting that moves FPS the most",
      },
      {
        arquivo: "espaco.png",
        titulo: "Space",
        descricao: "what can be freed — and, before that, where the disk actually went",
      },
      {
        arquivo: "sistema.png",
        titulo: "System",
        descricao: "what starts with Windows, across the three places it hides in",
      },
      {
        arquivo: "reparo.png",
        titulo: "Repair",
        descricao: "the Windows tools that return damaged system files to the original",
      },
    ],
  },

  recusas: {
    rotulo: "WHAT IT REFUSES TO DO",
    titulo: "Five things it doesn't do, and the reason for each one.",
    itens: [
      {
        titulo: "Doesn't turn off the Spectre/Meltdown protections",
        porque: "It would yield real FPS. It would also leave your machine exposed to a known flaw.",
      },
      {
        titulo: "Doesn't touch Windows Update, Defender or the firewall",
        porque:
          "It's the favorite tweak of the optimizers out there, and it's what turns the PC into a target.",
      },
      {
        titulo: "Doesn't do “registry cleaning”",
        porque: "There is no measurable gain and it breaks installed programs. It's expensive theater.",
      },
      {
        titulo: "Doesn't force-free RAM",
        porque:
          "It makes the graph look good and the PC slower — Windows drops from cache what it was about to use.",
      },
      {
        titulo: "Doesn't write to the BIOS",
        porque:
          "On consumer boards, a mistake there bricks the motherboard. It reads, points to where to fix it, and stops there.",
      },
    ],
  },

  privacidade: {
    rotulo: "PRIVACY",
    titulo: "It doesn't send anything of yours anywhere. One question goes out, and that is all.",
    texto:
      "The only thing Otimiza sends out is a question to GitHub: is there a new version? It is " +
      "anonymous, carries nothing from your machine, and without an answer the program simply " +
      "doesn't mention it and keeps working. There is no server of ours to send to, and no " +
      "telemetry to switch off in the options — because there is none.",
    itens: [
      "The source code is public and can be read by anyone who installs it",
      "The license is checked on your machine, without contacting a server",
      "Verification uses Ed25519: the program carries only the key that verifies, never the one that signs",
    ],
  },

  licenca: {
    rotulo: "HOW THE LICENSE WORKS",
    titulo: "One key, one computer — and formatting doesn't cost a new key.",
    itens: [
      {
        titulo: "Tied to the motherboard serial number",
        texto:
          "That is why reinstalling Windows doesn't invalidate your key. It keeps working on the same PC.",
      },
      {
        titulo: "Replacing the motherboard changes the code",
        texto:
          "Then the key stops working, and you contact me for a free reissue. It is written here " +
          "because you need to know it before buying, not when it happens.",
      },
      {
        titulo: "Lifetime, no monthly fee",
        texto:
          "Once you buy it, it's yours. There is no subscription, no renewal, no recurring charge.",
      },
    ],
    fluxoRotulo: "FROM INSTALL TO THE PASTED KEY",
    fluxo: [
      {
        passo: "01",
        titulo: "Install it and open it",
        texto:
          "Otimiza shows this machine's code, in the format OTZ-XXXX-XXXX-XXXX. The full diagnostic already runs here, before any payment.",
      },
      {
        passo: "02",
        titulo: "Pay and send the code",
        /* CONFIRMAR COM O DONO — same open question as in pt.ts: the delivery
           channel is not defined yet. This describes the mechanism, which is
           true, without promising a channel. */
        texto:
          "The key is issued for that specific code, with an Ed25519 signature. The delivery channel is given at checkout.",
      },
      {
        passo: "03",
        titulo: "Paste the key",
        texto:
          "The check happens inside the program, on your machine. No server is contacted, and there is no online activation.",
      },
    ],
    nota:
      "Before activation, Otimiza measures your machine and shows the main problem it found " +
      "on the purchase screen — measured right there, not advertising copy. The full program " +
      "opens with the key.",
  },

  precos: {
    rotulo: "PRICE",
    titulo: "One lifetime license, for one computer.",
    texto:
      "There is no monthly plan, no annual plan, and no “Pro” version with the same program and one " +
      "extra button. One product, one price, and it's yours.",

    planoRotulo: "LIFETIME LICENSE",
    planoTitulo: "Otimiza for 1 computer",
    planoResumo: "One-time payment. No subscription, no renewal, no recurring charge.",
    ctaTexto: "Buy",


    formasRotulo: "PAYMENT METHODS",
    /* Pix and Boleto are Brazilian payment instruments and keep their names —
       there is no English equivalent, and renaming them would hide the fact
       that they may not be available to a buyer outside Brazil.
       CONFIRMAR COM O DONO — see notaMoeda below. */
    formas: ["Pix", "Credit card", "Boleto"],
    processadoPor: "Processed by Mercado Pago.",
    requisito: "Windows 10 or 11, 64-bit.",

    incluiRotulo: "WHAT YOU NEED TO KNOW BEFORE PAYING",
    inclui: [
      {
        titulo: "One-time payment, no subscription",
        texto: "The license is for life. Once you buy it, it's yours — no renewal and no monthly fee.",
      },
      {
        titulo: "Valid for 1 computer",
        texto:
          "The key is created tied to the motherboard serial number. It won't open on another machine, and that is what it exists for.",
      },
      {
        titulo: "Formatting Windows doesn't invalidate the key",
        texto:
          "The machine code comes from the motherboard, which survives formatting. Reinstalling Windows doesn't cost a new key.",
      },
      {
        titulo: "Replaced the motherboard? The reissue is free",
        texto:
          "Replacing the board changes the machine code and the key stops working. You send the new code and receive another key, at no cost.",
      },
      {
        titulo: "Verification done on your machine",
        texto:
          "The key is checked locally, without contacting any server — not at activation, and not afterwards.",
      },
      {
        titulo: "Every change is reversible",
        texto:
          "Every change records the previous state before writing, and undo restores it identical, byte for byte.",
      },
    ],

    parcelamento: (parcelas, valor) =>
      `or ${parcelas}× ${valor}, interest-free, on a credit card`,

    /* CONFIRMAR COM O DONO — o preco fica em REAL (BRL) nas tres linguas,
       porque e isso que o Mercado Pago cobra. Converter para dolar na tela
       seria mentir sobre o valor cobrado. O que ainda NAO foi decidido:
       se o produto vai mesmo ser vendido fora do Brasil, e se o checkout do
       Mercado Pago aceita cartao internacional (Pix e Boleto nao servem para
       comprador de fora). Enquanto nao houver decisao, o site diz a verdade
       sobre a moeda e nada alem disso. */
    sufixoMoeda: "BRL",
    notaMoeda: "Charged in Brazilian reais (BRL) by Mercado Pago.",

    adicional: {
      rotulo: "OPTIONAL",
      titulo: "Additional license for another PC",
      texto:
        "Each computer needs its own key. If you want to install it on a second PC — the work one, your kid's — that is one more license, also for life.",
      /* CONFIRMAR COM O DONO — is there a discount on the second license?
         See PRECO_LICENCA_ADICIONAL_BRL in src/data/precos.ts. */
      observacao: "It is not required. A single license is enough if you have one PC.",
      porComputador: "per computer",
      comprar: "Buy",
    },
  },

  faq: {
    rotulo: "QUESTIONS",
    titulo: "Before you pay.",
    texto:
      "The questions that come up in support, answered here — including the ones that don't help the sale.",
    itens: [
      {
        pergunta: "Is this one more optimizer that promises FPS and delivers nothing?",
        resposta: [
          "That is exactly what the product was built against. Otimiza measures the machine before and after each change and shows the number — including when the number says nothing changed.",
          "The thresholds that separate gain from noise were not guessed: they came from a test that measures the same machine three times without changing anything. Whatever fluctuated there is noise, and it is never reported as a gain. Two of the six metrics produce no verdict at all, because calibration proved they vary too much on their own.",
        ],
      },
      {
        pergunta: "What if nothing improves on my PC?",
        resposta: [
          "It will say so, with a number. If the setting is already applied, the screen shows “already optimized” instead of faking work. And if the PC is busy — above 25% CPU usage — no verdict is issued, because comparing a busy PC with a rested one invents a gain.",
          "Worth knowing before you decide: on install, before any payment, Otimiza measures your machine and shows the main problem it found on the purchase screen — measured right there, not advertising copy. The full program opens with the key.",
        ],
      },
      {
        pergunta: "Is it safe? Can this break my Windows?",
        resposta: [
          "Every change records the previous state before writing, and undo restores it identical, byte for byte — not something equivalent. When Otimiza cannot read the previous state, it refuses to act: without knowing what was there before, there is no way to promise the undo.",
          "It also refuses to do what this category usually does: it doesn't turn off the Spectre/Meltdown protections, doesn't touch Windows Update, Defender or the firewall, doesn't do “registry cleaning”, doesn't force-free RAM and doesn't write to the BIOS.",
        ],
      },
      {
        pergunta: "Do I need internet to use it?",
        resposta: [
          "No. There is no online activation and no periodic license check: once installed, it works with the machine disconnected. The only thing it does over the internet is ask GitHub whether a new version is out — and without an answer, it simply doesn't mention it.",
        ],
      },
      {
        pergunta: "Do you collect my data?",
        resposta: [
          "No — and this is not a promise you have to believe. There is no server to send to and no telemetry to switch off in the options: it is something the program is unable to do.",
          "The source code is public and can be read by anyone who installs it. License verification uses Ed25519, and the program carries only the key that verifies a signature — never the one that signs. It doesn't contact any server to find out whether you may use it.",
        ],
      },
      {
        pergunta: "I formatted Windows. Did I lose my key?",
        resposta: [
          "No. The machine code comes from the motherboard serial number, and that is the reason for the choice: it survives formatting. Reinstall Windows, install Otimiza and paste the same key.",
          "There is one exception, and the program warns you about it on screen: on some machines the manufacturer leaves the board serial number blank. In those cases the code is derived from the Windows identifier, and then formatting changes the code. If that happens to you, the reissue is free as well.",
        ],
      },
      {
        pergunta: "I replaced the motherboard. What now?",
        resposta: [
          "The key stops working, because the machine code changed. You send the new code and receive another key, at no cost.",
          "This is written here, before the purchase, because you need to know it now — and not on the day it happens.",
        ],
      },
      {
        pergunta: "Can I use the same key on two computers?",
        resposta: [
          "No. Each key works on one machine only, and that is exactly what it exists to do. Passing your key to someone else doesn't work — it won't open on another PC.",
          "For a second computer, it is an additional license, also for life.",
        ],
      },
      {
        pergunta: "Does it work on Windows 10?",
        resposta: [
          "Yes. Windows 10 or 11, 64-bit. There is no macOS or Linux version: what Otimiza does depends on the Windows Registry and on Windows services.",
        ],
      },
      {
        pergunta: "Why does Windows say “unknown publisher” when I install it?",
        resposta: [
          "Because the installer doesn't have a digital signature yet. SmartScreen shows “Windows protected your PC” on the first run; just click “More info” and then “Run anyway”.",
          "This is written in the notes of every version instead of hidden. A code signing certificate requires a purchase and an identity check, and that hasn't been done yet.",
        ],
      },
      {
        pergunta: "How do I get the key after paying?",
        /* CONFIRMAR COM O DONO — delivery channel still undefined; this text
           describes only the mechanism, which is true. */
        confirmar: true,
        resposta: [
          "The key is issued for your machine's code, so the step is: install Otimiza, copy the code shown on the screen — in the format OTZ-XXXX-XXXX-XXXX — and send it along with the payment. The key comes back ready to paste into the field.",
          "The sending instructions and the turnaround appear at checkout.",
        ],
      },
      {
        pergunta: "Is there a refund?",
        /* CONFIRMAR COM O DONO — refund policy is a commercial and legal
           decision. Do NOT invent a deadline, a condition or a guarantee here,
           in any language. */
        confirmar: true,
        resposta: [
          "The refund policy will be stated on the purchase page, before the payment is confirmed.",
        ],
      },
    ],
  },

  comprar: {
    tituloPagina: "Buy Otimiza",
    descricaoPagina:
      "Four steps. You install first, because the key is bound to the computer — and its code only appears after installing.",
    rotulo: "BUY",
    titulo: "Install first. That is not red tape — it is the only order that works.",
    texto:
      "Your key is bound to this computer, and its code only exists after Otimiza has run here once. " +
      "Paying first would leave you with a key that belongs to nobody and nothing to paste it into. The upside: when it opens, " +
      "before you pay, it already measures your machine and shows the main problem it found on it.",
    passos: [
      {
        numero: "01",
        titulo: "Download and install",
        texto:
          "Free, no sign-up. Windows will say “unknown publisher” — that is expected, the installer is not code-signed yet. Click More info, then Run anyway.",
      },
      {
        numero: "02",
        titulo: "Open it and copy your code",
        texto:
          "On first launch it shows the code for this computer, shaped OTZ-XXXX-XXXX-XXXX, along with the main problem it found on your machine. Copy the code.",
      },
      {
        numero: "03",
        titulo: "Send the code and pay",
        texto:
          "On Discord, send the code and pick how to pay: Pix, credit card or boleto, through Mercado Pago. The key is issued for that code and no other.",
      },
      {
        numero: "04",
        titulo: "Paste the key",
        texto:
          "Back in Otimiza, paste it into the “Your key” field and click Activate. Verification happens on your machine, with no server involved. Done — it is permanent.",
      },
    ],
    acaoBaixar: "Download for Windows",
    acaoDiscord: "Go to Discord",
    passoPagarAqui:
      "Paste the code below and pay with Pix. The key is issued for that code and appears on this same screen when the payment lands — usually within seconds.",
    notaDiscord:
      "Support happens there because a person issues the key at the moment you pay. No bot decides it.",
    conferidor: {
      rotulo: "BEFORE YOU PAY",
      titulo: "Check that the code was copied correctly.",
      texto:
        "The worst thing that can happen in this purchase is paying and receiving a key that will not open, because the code was mistyped. Paste it here and I will check its shape.",
      etiqueta: "The code Otimiza showed you",
      botao: "Check",
      certo: "The shape is right. You can send this code on Discord.",
      errado:
        "That is not shaped like an Otimiza code. It is OTZ- followed by three blocks of four characters, like this: OTZ-XXXX-XXXX-XXXX. Copy it again straight from the program.",
      vazio: "Paste the code so I can check it.",
      aviso:
        "This checks the SHAPE only. From here I cannot know whether the code really belongs to your computer — that check happens when the key is issued.",
    },
    voltar: "Back to the home page",
  },

  checkout: {
    rotulo: "PAY NOW",
    titulo: "Paste your computer code and pay with Pix.",
    texto:
      "The key is issued for that code and arrives right here, on this screen, as soon as the payment lands. " +
      "It usually takes a few seconds.",

    etiquetaCodigo: "The code Otimiza showed you",
    gerar: "Generate the Pix",
    gerando: "Generating…",

    pixTitulo: "Pay this Pix",
    pixTexto:
      "Open your bank app, choose Pix, and scan the code or paste the copy-and-paste string. " +
      "Do not close this page: the key appears here when the payment lands.",
    copiar: "Copy the Pix code",
    copiado: "Copied",
    expiraEm: "This Pix is valid for another",
    expirou: "This Pix expired. Nothing was charged — generate another whenever you want.",
    aguardando: "Waiting for the payment…",

    prontoTitulo: "Done. This is your key.",
    prontoTexto:
      "It lasts forever, on this computer. Paste it into the “Your key” field in Otimiza and click Activate.",
    chaveEtiqueta: "Your key",
    copiarChave: "Copy the key",
    guardeAChave:
      "Keep a copy. If you lose it, the key is reissued at no cost — but you will need to contact support.",

    erroGenerico: "Could not finish right now. Nothing was charged.",
    erroRede: "Could not reach the server. Nothing was charged — check your connection and try again.",
    erroForaDoAr:
      "Buying through the site is offline right now. Nothing was charged — the Discord route still works.",
    erroEstornada: "This payment was refunded. Contact support if that was not you.",
    recomecar: "Start over",

    ouEntaoTitulo: "Rather talk to a person?",
    ouEntaoTexto:
      "Discord is still a complete purchase route, with a human on the other side — and it is the same price.",

    semScript:
      "Paying on this page needs JavaScript, which is turned off in your browser. You can buy through Discord using the button below — same price, same key.",
  },

  rodape: {
    links: [
      { href: "#pilares", texto: "How it works" },
      { href: "#telas", texto: "The screens" },
      { href: "#recusas", texto: "What it won't do" },
      { href: "#licenca", texto: "The license" },
      { href: "#privacidade", texto: "Privacy" },
      { href: "#precos", texto: "Price" },
    ],
    /* Do NOT claim here that the source code is public or auditable — that
       claim was removed on purpose. */
    direitos: "© {ano} Otimiza. Source open to reading, not to use.",
  },
};
