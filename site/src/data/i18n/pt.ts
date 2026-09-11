/**
 * PORTUGUES DO BRASIL — a fonte da verdade.
 *
 * Este arquivo e o original. en.ts e es.ts sao traducoes DELE, e nao versoes
 * independentes: quando o texto mudar aqui, muda nos outros dois.
 *
 * REGRA HERDADA DO conteudo.ts ORIGINAL: todo numero aqui saiu de uma medicao
 * registrada no repositorio (README.md, .github/release-notes.md,
 * pc-optimizer/PROGRESS.md). Nenhum foi arredondado para soar melhor, e nenhum
 * foi inventado. O campo `fonte` diz de onde cada um veio.
 *
 * Isso nao e preciosismo: o produto inteiro se vende dizendo que o mercado
 * anuncia numero que ninguem consegue conferir. Uma landing com numero
 * inflado destruiria o unico argumento que o Otimiza tem.
 */
import type { Conteudo } from "./tipos";

export const pt: Conteudo = {
  meta: {
    htmlLang: "pt-BR",
    hreflang: "pt-BR",
    ogLocale: "pt_BR",
    localeNumero: "pt-BR",
    sigla: "PT",
    nome: "Português",
    titulo: "Otimiza — Console de desempenho para Windows",
    descricao:
      "Mede o que o seu PC faz, otimiza o que dá, e prova com número — inclusive quando o número diz que não mudou nada.",
  },

  a11y: {
    pularConteudo: "Pular para o conteúdo",
    secoes: "Seções",
    abrirMenu: "Abrir menu de seções",
    trocarTema: "Alternar entre tema claro e escuro",
    idioma: "Idioma",
    linksRodape: "Links do rodapé",
    numerosMedidos: "Números medidos",
    capturaPendente: "Captura pendente",
  },

  cabecalho: {
    links: [
      { href: "#pilares", texto: "Como funciona" },
      { href: "#recusas", texto: "O que ele não faz" },
      { href: "#privacidade", texto: "Privacidade" },
    ],
    comprar: "Comprar",
    menu: "Menu",
    preco: "Preço",
  },

  hero: {
    promessa: "Console de desempenho para Windows",
    chamada:
      "Mede o que o seu PC faz, otimiza o que dá, e prova com número — inclusive quando o número diz que não mudou nada.",
    texto:
      "Barra de progresso, lista de ajustes copiada da internet, e um ganho inventado no final. " +
      "O Otimiza é construído em cima da recusa a fazer isso.",
    ctaBaixar: "Baixar para Windows",
    ctaComprar: "Ver preço",
    ctaTelas: "Ver as telas",
    versaoPrefixo: "Versão",
    linhaMeta:
      "Grátis para baixar · ativação R$ 25, uma vez · Windows 10 e 11, 64 bits",
    captura: {
      titulo: "Painel",
      descricao: "a abertura: o que está travando este PC, medido na hora e dito na cara",
    },
  },

  numeros: [
    { valor: "677", legenda: "testes automatizados, zero avisos", fonte: "commit da 1.7.0" },
    { valor: "1,2 s", legenda: "para abrir, de 3,7 s na versão anterior", fonte: "notas da 1.7.0" },
    { valor: "byte a byte", legenda: "a precisão do desfazer", fonte: "PROGRESS.md" },
    { valor: "zero", legenda: "dados seus que saem da sua máquina", fonte: "só sai a pergunta de versão, anônima" },
  ],

  pilares: {
    rotulo: "O PROBLEMA",
    titulo: "Todo otimizador de PC anuncia um número que você não tem como conferir.",
    itens: [
      {
        numero: "01",
        rotulo: "MEDIÇÃO",
        titulo: "Mede antes e depois — e admite quando não mudou nada",
        texto:
          "Os limiares de ruído não foram chutados: vieram de um teste que mede a mesma máquina três " +
          "vezes sem alterar coisa alguma. O que variou ali é ruído, e nunca é reportado como ganho.",
        detalhe:
          "Duas das seis métricas nem geram veredito — a calibração provou que oscilam demais sozinhas.",
        fonte: "README.md · noise_calibration",
      },
      {
        numero: "02",
        rotulo: "A TRAVADA",
        titulo: "Mede o engasgo, não a média de FPS",
        texto:
          "Ninguém reclama de média baixa — reclama que o jogo trava. Um congelamento de 40 ms arruína " +
          "a suavidade e quase não mexe na média de 60 quadros por segundo.",
        detalhe: "O Otimiza lê o atraso do agendador do Windows direto, que é o que o jogador sente.",
        fonte: "README.md · módulo jitter",
      },
      {
        numero: "03",
        rotulo: "RECUSA",
        titulo: "Se recusa a medir com o PC ocupado",
        texto:
          "Comparar um PC ocupado com um PC descansado inventa um ganho de dezenas por cento. " +
          "Acima de 25% de uso de CPU, nenhum veredito é emitido.",
        detalhe: "E a tela explica por quê, em vez de mostrar um número que não vale.",
        fonte: "README.md",
      },
      {
        numero: "04",
        rotulo: "HARDWARE",
        titulo: "Lê a sua máquina antes de oferecer",
        texto:
          "Desativar o SysMain ajuda em SSD e atrapalha em disco mecânico. Desligar a compressão de " +
          "memória ajuda com RAM sobrando e piora com 8 GB.",
        detalhe: "O Otimiza detecta o hardware e não oferece o que faria mal a ele.",
        fonte: "README.md",
      },
      {
        numero: "05",
        rotulo: "REVERSÍVEL",
        titulo: "Desfazer restaura idêntico, byte a byte",
        texto:
          "Cada mudança grava o estado anterior antes de escrever. Desfazer não restaura algo " +
          "equivalente: restaura exatamente o que estava lá.",
        detalhe:
          "O ciclo completo foi executado contra o registro do Windows e conferido por fora, com PowerShell.",
        fonte: "PROGRESS.md · ciclo real",
      },
      {
        numero: "06",
        rotulo: "JÁ OTIMIZADO",
        titulo: "Diz quando não há nada a fazer",
        texto:
          "Se a configuração já está aplicada, ele mostra “já otimizado” em vez de fingir trabalho " +
          "e cobrar o crédito por uma mudança que não aconteceu.",
        detalhe: null,
        fonte: "README.md",
      },
    ],
    provaRotulo: "A PROVA",
    provaTitulo: "O antes e o depois, lado a lado, com o veredito de cada métrica.",
    provaCaptura: {
      titulo: "Otimizações",
      descricao: "medir antes, otimizar, medir de novo — e o catálogo dizendo o que já está otimizado",
    },
  },

  telas: {
    rotulo: "AS TELAS",
    titulo: "O programa inteiro, sem tela escondida.",
    itens: [
      {
        arquivo: "diagnostico.png",
        titulo: "Diagnóstico",
        descricao: "monitores, memória instalada e os achados desta máquina",
      },
      {
        arquivo: "jogos.png",
        titulo: "Jogos",
        descricao: "a placa de vídeo, o driver, e a configuração do jogo que mais mexe no FPS",
      },
      {
        arquivo: "espaco.png",
        titulo: "Espaço",
        descricao: "o que dá para liberar — e, antes disso, onde o disco realmente foi parar",
      },
      {
        arquivo: "sistema.png",
        titulo: "Sistema",
        descricao: "o que sobe junto com o Windows, nos três lugares onde isso se esconde",
      },
      {
        arquivo: "reparo.png",
        titulo: "Reparo",
        descricao: "as ferramentas do Windows que devolvem arquivo de sistema danificado ao original",
      },
    ],
  },

  recusas: {
    rotulo: "O QUE ELE SE RECUSA A FAZER",
    titulo: "Cinco coisas que ele não faz, e o motivo de cada uma.",
    itens: [
      {
        titulo: "Não desliga as proteções contra Spectre/Meltdown",
        porque: "Renderia FPS real. Também deixaria a sua máquina exposta a uma falha conhecida.",
      },
      {
        titulo: "Não mexe no Windows Update, no Defender nem no firewall",
        porque: "É o ajuste preferido dos otimizadores de internet, e é o que transforma o PC em alvo.",
      },
      {
        titulo: "Não faz “limpeza de registro”",
        porque: "Não tem ganho medível e quebra programa instalado. É teatro caro.",
      },
      {
        titulo: "Não libera RAM à força",
        porque: "Deixa o gráfico bonito e o PC mais lento — o Windows tira do cache o que ia usar.",
      },
      {
        titulo: "Não escreve na BIOS",
        porque: "Em placa de consumo, errar ali inutiliza a placa-mãe. Ele lê, aponta onde resolver, e para aí.",
      },
    ],
  },

  privacidade: {
    rotulo: "PRIVACIDADE",
    titulo: "Ele não manda nada seu para lugar nenhum. Sai uma pergunta, e só.",
    texto:
      "A única coisa que o Otimiza envia para fora é uma pergunta ao GitHub: saiu versão nova? Ela " +
      "é anônima, não leva nada da sua máquina, e sem resposta o programa apenas não avisa e segue " +
      "funcionando. Não há servidor nosso para onde mandar, e não há telemetria para desligar nas " +
      "opções — porque não existe nenhuma.",
    itens: [
      "O código-fonte é público e pode ser lido por quem instala",
      "A licença é conferida na sua máquina, sem consultar servidor",
      "A verificação usa Ed25519: o programa carrega só a chave que confere, nunca a que assina",
    ],
  },

  licenca: {
    rotulo: "COMO A LICENÇA FUNCIONA",
    titulo: "Uma chave, um computador — e formatar não custa chave nova.",
    itens: [
      {
        titulo: "Presa ao número de série da placa-mãe",
        texto:
          "É por isso que formatar o Windows não invalida a sua chave. Ela continua valendo no mesmo PC.",
      },
      {
        titulo: "Trocar a placa-mãe muda o código",
        texto:
          "Aí a chave para de valer, e você me chama para reemitir sem custo. Está escrito aqui porque " +
          "você precisa saber disso antes de comprar, não quando acontecer.",
      },
      {
        titulo: "Vitalícia, sem mensalidade",
        texto: "Comprou, é seu. Não há assinatura, não há renovação, não há cobrança recorrente.",
      },
    ],
    fluxoRotulo: "DA INSTALAÇÃO ATÉ A CHAVE COLADA",
    fluxo: [
      {
        passo: "01",
        titulo: "Instale e abra",
        texto:
          "O Otimiza mostra o código desta máquina, no formato OTZ-XXXX-XXXX-XXXX. O diagnóstico completo já roda aqui, antes de qualquer pagamento.",
      },
      {
        passo: "02",
        titulo: "Pague e envie o código",
        /* CONFIRMAR COM O DONO — o canal de entrega (Discord, e-mail, painel do
           checkout) ainda nao esta definido. O texto abaixo descreve o mecanismo,
           que e verdadeiro, sem prometer um canal. */
        texto:
          "A chave é emitida para esse código específico, com assinatura Ed25519. O canal de envio é informado no checkout.",
      },
      {
        passo: "03",
        titulo: "Cole a chave",
        texto:
          "A conferência acontece dentro do programa, na sua máquina. Não há servidor consultado, nem ativação online.",
      },
    ],
    nota:
      "Antes de ativar, o Otimiza mede a sua máquina e mostra na tela de compra o principal " +
      "problema que encontrou nela — medido na hora, não texto de propaganda. O programa " +
      "completo abre com a chave.",
  },

  precos: {
    rotulo: "PREÇO",
    titulo: "Uma licença vitalícia, para um computador.",
    texto:
      "Não há plano mensal, plano anual, nem versão “Pro” com o mesmo programa e um botão a mais. " +
      "É um produto, um preço, e ele é seu.",

    planoRotulo: "LICENÇA VITALÍCIA",
    planoTitulo: "Otimiza para 1 computador",
    planoResumo: "Pagamento único. Sem assinatura, sem renovação, sem cobrança recorrente.",
    ctaTexto: "Comprar",


    formasRotulo: "FORMAS DE PAGAMENTO",
    formas: ["Pix", "Cartão de crédito", "Boleto"],
    processadoPor: "Processado pelo Mercado Pago.",
    requisito: "Windows 10 ou 11, 64 bits.",

    incluiRotulo: "O QUE VOCÊ PRECISA SABER ANTES DE PAGAR",
    inclui: [
      {
        titulo: "Pagamento único, sem assinatura",
        texto: "A licença é vitalícia. Comprou, é seu — não há renovação nem mensalidade.",
      },
      {
        titulo: "Vale para 1 computador",
        texto:
          "A chave nasce presa ao número de série da placa-mãe. Ela não abre em outra máquina, e é para isso que ela existe.",
      },
      {
        titulo: "Formatar o Windows não invalida a chave",
        texto:
          "O código da máquina vem da placa-mãe, que sobrevive à formatação. Reinstalar o Windows não custa chave nova.",
      },
      {
        titulo: "Trocou a placa-mãe? A reemissão é gratuita",
        texto:
          "Trocar a placa muda o código da máquina e a chave para de valer. Você manda o código novo e recebe outra chave, sem custo.",
      },
      {
        titulo: "Verificação feita na sua máquina",
        texto:
          "A chave é conferida localmente, sem consultar servidor nenhum — nem na ativação, nem depois.",
      },
      {
        titulo: "Todas as alterações são reversíveis",
        texto:
          "Cada mudança grava o estado anterior antes de escrever, e o desfazer restaura idêntico, byte a byte.",
      },
    ],

    parcelamento: (parcelas, valor) => `ou ${parcelas}x de ${valor} sem juros no cartão`,

    /* Em portugues "R$" ja diz tudo: quem le esta pagina paga em real e sabe
       disso. O aviso de moeda existe so nas outras duas linguas. */
    sufixoMoeda: null,
    notaMoeda: null,

    adicional: {
      rotulo: "OPCIONAL",
      titulo: "Licença adicional para outro PC",
      texto:
        "Cada computador precisa da sua própria chave. Se você quiser instalar em um segundo PC — o do trabalho, o do filho —, é uma licença a mais, também vitalícia.",
      /* CONFIRMAR COM O DONO — ha desconto na segunda licenca? Ver
         PRECO_LICENCA_ADICIONAL_BRL em src/data/precos.ts. */
      observacao: "Não é obrigatório. A licença única já resolve para quem tem um PC.",
      porComputador: "por computador",
      comprar: "Comprar",
    },
  },

  faq: {
    rotulo: "PERGUNTAS",
    titulo: "Antes de você pagar.",
    texto:
      "As dúvidas que aparecem no suporte, respondidas aqui — inclusive as que não favorecem a venda.",
    itens: [
      {
        pergunta: "Isso é mais um otimizador que promete FPS e não entrega nada?",
        resposta: [
          "É contra isso que o produto foi construído. O Otimiza mede a máquina antes e depois de cada alteração e mostra o número — inclusive quando o número diz que não mudou nada.",
          "Os limiares que separam ganho de ruído não foram chutados: vieram de um teste que mede a mesma máquina três vezes sem alterar coisa alguma. O que oscilou ali é ruído, e nunca é reportado como ganho. Duas das seis métricas nem geram veredito, porque a calibração provou que variam demais sozinhas.",
        ],
      },
      {
        pergunta: "E se não melhorar nada no meu PC?",
        resposta: [
          "Ele vai dizer isso, com número. Se a configuração já estiver aplicada, a tela mostra “já otimizado” em vez de fingir trabalho. E se o PC estiver ocupado — acima de 25% de uso de CPU — nenhum veredito é emitido, porque comparar um PC ocupado com um PC descansado inventa ganho.",
          "Vale saber antes de decidir: ao instalar, antes de qualquer pagamento, o Otimiza mede a sua máquina e mostra na tela de compra o principal problema que encontrou nela — medido na hora, não texto de propaganda. O programa completo abre com a chave.",
        ],
      },
      {
        pergunta: "É seguro? Isso pode quebrar meu Windows?",
        resposta: [
          "Toda alteração grava o estado anterior antes de escrever, e o desfazer restaura idêntico, byte a byte — não algo equivalente. Quando o Otimiza não consegue ler o estado anterior, ele se recusa a agir: sem saber o que havia antes, não há como prometer o desfazer.",
          "Ele também se recusa a fazer o que a categoria costuma fazer: não desliga as proteções contra Spectre/Meltdown, não mexe no Windows Update, no Defender nem no firewall, não faz “limpeza de registro”, não libera RAM à força e não escreve na BIOS.",
        ],
      },
      {
        pergunta: "Preciso de internet para usar?",
        resposta: [
          "Não. Não há ativação online nem verificação periódica de licença: depois de instalado, ele funciona com a máquina desconectada. A única coisa que ele faz pela internet é perguntar ao GitHub se saiu versão nova — e sem resposta, ele apenas não avisa.",
        ],
      },
      {
        pergunta: "Vocês coletam meus dados?",
        resposta: [
          "Não — e isso não é uma promessa que você precisa acreditar. Não existe servidor para onde mandar nem telemetria para desligar nas opções: é uma coisa que o programa não consegue fazer.",
          "O código-fonte é público e pode ser lido por quem instala. A verificação da licença usa Ed25519, e o programa carrega apenas a chave que confere assinatura — nunca a que assina. Ele não consulta servidor nenhum para saber se você pode usar.",
        ],
      },
      {
        pergunta: "Formatei o Windows. Perdi a minha chave?",
        resposta: [
          "Não. O código da máquina vem do número de série da placa-mãe, e é essa a razão da escolha: ele sobrevive à formatação. Reinstale o Windows, instale o Otimiza e cole a mesma chave.",
          "Há uma exceção, e o programa te avisa dela na tela: em algumas máquinas o fabricante deixa o número de série da placa em branco. Nesses casos o código é derivado do identificador do Windows, e aí formatar muda o código. Se isso acontecer com você, a reemissão também é gratuita.",
        ],
      },
      {
        pergunta: "Troquei a placa-mãe. E agora?",
        resposta: [
          "A chave para de valer, porque o código da máquina mudou. Você manda o código novo e recebe outra chave, sem custo.",
          "Isso está escrito aqui, antes da compra, porque você precisa saber disso agora — e não no dia em que acontecer.",
        ],
      },
      {
        pergunta: "Posso usar a mesma chave em dois computadores?",
        resposta: [
          "Não. Cada chave vale em uma máquina só, e é exatamente isso que ela existe para fazer. Passar a sua chave para outra pessoa não funciona — ela não abre em outro PC.",
          "Para um segundo computador, é uma licença adicional, também vitalícia.",
        ],
      },
      {
        pergunta: "Funciona no Windows 10?",
        resposta: [
          "Sim. Windows 10 ou 11, 64 bits. Não há versão para macOS ou Linux: o que o Otimiza faz depende do registro e dos serviços do Windows.",
        ],
      },
      {
        pergunta: "Por que o Windows diz “editor desconhecido” quando eu instalo?",
        resposta: [
          "Porque o instalador ainda não tem assinatura digital. O SmartScreen mostra “O Windows protegeu o seu PC” na primeira execução; é só clicar em “Mais informações” e depois em “Executar assim mesmo”.",
          "Isso está escrito nas notas de cada versão em vez de escondido. O certificado de assinatura de código exige compra e verificação de identidade, e ainda não foi feito.",
        ],
      },
      {
        pergunta: "Como recebo a chave depois de pagar?",
        /* CONFIRMAR COM O DONO — o canal de entrega ainda nao esta definido
           (LICENCA.md descreve o fluxo pelo Discord; o site pode mudar isso).
           O texto abaixo descreve so o mecanismo, que e verdadeiro. */
        confirmar: true,
        resposta: [
          "A chave é emitida para o código da sua máquina, então o passo é: instalar o Otimiza, copiar o código que aparece na tela — no formato OTZ-XXXX-XXXX-XXXX — e enviá-lo junto com o pagamento. A chave volta pronta para colar no campo.",
          "As instruções de envio e o prazo aparecem no checkout.",
        ],
      },
      {
        pergunta: "Tem reembolso?",
        /* CONFIRMAR COM O DONO — politica de reembolso e decisao comercial e
           juridica. NAO inventar prazo, condicao nem garantia aqui. O Codigo de
           Defesa do Consumidor tem regra propria para compra a distancia; o dono
           precisa definir o texto com quem for cuidar disso. */
        confirmar: true,
        resposta: [
          "A política de reembolso será informada na página de compra, antes da confirmação do pagamento.",
        ],
      },
    ],
  },

  comprar: {
    tituloPagina: "Comprar o Otimiza",
    descricaoPagina:
      "Quatro passos. Você instala primeiro, porque a chave nasce presa ao computador — e o código dele só aparece depois de instalar.",
    rotulo: "COMPRAR",
    titulo: "Instale primeiro. Não é burocracia — é a única ordem que funciona.",
    texto:
      "A sua chave nasce presa a este computador, e o código dele só existe depois que o Otimiza roda aqui uma vez. " +
      "Pagar antes deixaria você com uma chave sem dono e sem nada para colar. O lado bom: ao abrir, antes de pagar, " +
      "ele já mede a sua máquina e mostra o principal problema que encontrou nela.",
    passos: [
      {
        numero: "01",
        titulo: "Baixe e instale",
        texto:
          "Grátis, sem cadastro. O Windows vai dizer “editor desconhecido” — é esperado, o instalador ainda não tem assinatura digital. Clique em Mais informações e depois em Executar assim mesmo.",
      },
      {
        numero: "02",
        titulo: "Abra e copie o seu código",
        texto:
          "Na primeira abertura aparece o código deste computador, no formato OTZ-XXXX-XXXX-XXXX, junto com o principal problema que ele achou na sua máquina. Copie o código.",
      },
      {
        numero: "03",
        titulo: "Mande o código e pague",
        texto:
          "No Discord, mande o código e escolha como pagar: Pix, cartão de crédito ou boleto, pelo Mercado Pago. A chave é emitida para esse código e para mais nenhum.",
      },
      {
        numero: "04",
        titulo: "Cole a chave",
        texto:
          "De volta no Otimiza, cole no campo “A sua chave” e clique em Ativar. A conferência acontece na sua máquina, sem consultar servidor. Pronto — é para sempre.",
      },
    ],
    acaoBaixar: "Baixar para Windows",
    acaoDiscord: "Ir para o Discord",
    passoPagarAqui:
      "Cole o código aqui embaixo e pague por Pix. A chave é emitida para esse código e aparece nesta mesma tela quando o pagamento cair — normalmente em segundos.",
    notaDiscord:
      "O atendimento é por lá porque quem emite a chave é uma pessoa, no momento em que você paga. Não há robô decidindo isso.",
    conferidor: {
      rotulo: "ANTES DE PAGAR",
      titulo: "Confira se o código está bem copiado.",
      texto:
        "O pior que pode acontecer nesta compra é você pagar e receber uma chave que não abre, porque o código foi copiado errado. Cole aqui que eu confiro a forma dele.",
      etiqueta: "O código que apareceu no Otimiza",
      botao: "Conferir",
      certo: "A forma está certa. Pode mandar este código no Discord.",
      errado:
        "Isso não tem a forma de um código do Otimiza. Ele é OTZ- seguido de três blocos de quatro caracteres, assim: OTZ-XXXX-XXXX-XXXX. Copie de novo direto da tela do programa.",
      vazio: "Cole o código para eu conferir.",
      aviso:
        "Isto confere só a FORMA. Não tenho como saber, daqui, se o código é mesmo o do seu computador — essa conferência acontece na hora de emitir a chave.",
    },
    voltar: "Voltar para a página inicial",
  },

  checkout: {
    rotulo: "PAGAR AGORA",
    titulo: "Cole o código do seu computador e pague por Pix.",
    texto:
      "A chave é emitida para esse código e chega aqui mesmo, nesta tela, assim que o pagamento cair. " +
      "Costuma levar poucos segundos.",

    etiquetaCodigo: "O código que apareceu no Otimiza",
    gerar: "Gerar o Pix",
    gerando: "Gerando…",

    pixTitulo: "Pague este Pix",
    pixTexto:
      "Abra o aplicativo do seu banco, escolha Pix, e leia o código ou cole o copia-e-cola. " +
      "Não feche esta página: a chave aparece aqui quando o pagamento cair.",
    copiar: "Copiar o código Pix",
    copiado: "Copiado",
    expiraEm: "Este Pix vale por mais",
    expirou: "Este Pix venceu. Nada foi cobrado — gere outro quando quiser.",
    aguardando: "Esperando o pagamento…",

    prontoTitulo: "Pronto. A sua chave é esta.",
    prontoTexto:
      "Ela vale para sempre, neste computador. Cole no campo “A sua chave” do Otimiza e clique em Ativar.",
    chaveEtiqueta: "A sua chave",
    copiarChave: "Copiar a chave",
    guardeAChave:
      "Guarde uma cópia. Se perder, a chave é reemitida sem custo — mas você vai precisar falar com o suporte.",

    erroGenerico: "Não deu para completar agora. Nada foi cobrado.",
    erroRede: "Não consegui falar com o servidor. Nada foi cobrado — confira sua conexão e tente de novo.",
    erroForaDoAr:
      "A compra pelo site está fora do ar neste momento. Nada foi cobrado — o caminho pelo Discord continua funcionando.",
    erroEstornada: "Este pagamento foi estornado. Fale com o suporte se isso não foi você.",
    recomecar: "Começar de novo",

    ouEntaoTitulo: "Prefere falar com uma pessoa?",
    ouEntaoTexto:
      "O Discord continua sendo um caminho completo de compra, com atendimento humano — e é o mesmo preço.",

    semScript:
      "O pagamento nesta página precisa de JavaScript, que está desligado no seu navegador. Você pode comprar pelo Discord, no botão abaixo — é o mesmo preço e a mesma chave.",
  },

  rodape: {
    links: [
      { href: "#pilares", texto: "Como funciona" },
      { href: "#telas", texto: "As telas" },
      { href: "#recusas", texto: "O que ele não faz" },
      { href: "#licenca", texto: "A licença" },
      { href: "#privacidade", texto: "Privacidade" },
      { href: "#precos", texto: "Preço" },
    ],
    /* O `{ano}` e substituido pelo componente. NAO afirmar aqui que o codigo
       e publico ou auditavel: essa afirmacao foi retirada de proposito. */
    direitos: "© {ano} Otimiza. Código aberto à leitura, não ao uso.",
  },
};
