/**
 * ESPANOL NEUTRO, LATINOAMERICANO — traducido de pt.ts, la fuente de verdad.
 *
 * REGLAS DE ESTE ARCHIVO, EN ORDEN DE IMPORTANCIA:
 *
 * 1. Ningun numero cambia. 677 pruebas, 25% de CPU, 40 ms, 8 GB son mediciones
 *    y una medicion no se traduce. Solo cambia el separador decimal, y en
 *    espanol es coma, igual que en portugues: "1,2 s".
 *
 * 2. No "mejorar" el texto al traducir. Si el portugues dice "admite cuando no
 *    cambio nada", el espanol dice eso, no "maximiza tu rendimiento". El
 *    producto entero es una acusacion contra un mercado que anuncia numeros
 *    que nadie puede verificar; marketing vacio aqui destruiria ese argumento.
 *
 * 3. Espanol de America Latina, no de Espana: "computadora" y no "ordenador",
 *    "clave" y no "llave", ustedes y nunca vosotros. El trato es de "usted",
 *    que es el registro neutro que funciona de Mexico a Argentina.
 *
 * 4. Terminos de Windows con su nombre real en espanol: "Registro de Windows",
 *    y los textos de SmartScreen son los que muestra el instalador en espanol.
 *
 * 5. Ojo con el pilar 02: "agendador do Windows" ahi es el planificador de
 *    hilos del sistema, NO el Programador de tareas. Traducirlo como
 *    "Programador de tareas" volveria falsa una frase verdadera.
 *
 * 6. El precio queda en reales brasilenos. Mercado Pago cobra en BRL; mostrar
 *    un valor en pesos seria mentir sobre lo que el checkout va a cobrar.
 */
import type { Conteudo } from "./tipos";

export const es: Conteudo = {
  meta: {
    htmlLang: "es",
    hreflang: "es",
    ogLocale: "es_ES",
    /* Etiqueta usada SOLO para formatear numeros. es-AR usa coma decimal, que
       es la convencion de la mayor parte de America Latina. No dice nada sobre
       el pais del lector: el texto es neutro. */
    localeNumero: "es-AR",
    sigla: "ES",
    nome: "Español",
    titulo: "Otimiza — Consola de rendimiento para Windows",
    descricao:
      "Mide lo que hace su computadora, optimiza lo que se puede, y lo demuestra con números — incluso cuando el número dice que no cambió nada.",
  },

  a11y: {
    pularConteudo: "Saltar al contenido",
    secoes: "Secciones",
    abrirMenu: "Abrir menú de secciones",
    trocarTema: "Alternar entre tema claro y oscuro",
    idioma: "Idioma",
    linksRodape: "Enlaces del pie de página",
    numerosMedidos: "Números medidos",
    capturaPendente: "Captura pendiente",
  },

  cabecalho: {
    links: [
      { href: "#pilares", texto: "Cómo funciona" },
      { href: "#recusas", texto: "Lo que no hace" },
      { href: "#privacidade", texto: "Privacidad" },
    ],
    comprar: "Comprar",
    menu: "Menú",
    preco: "Precio",
  },

  hero: {
    promessa: "Consola de rendimiento para Windows",
    chamada:
      "Mide lo que hace su computadora, optimiza lo que se puede, y lo demuestra con números — incluso cuando el número dice que no cambió nada.",
    texto:
      "Una barra de progreso, una lista de ajustes copiada de internet, y una ganancia inventada al " +
      "final. Otimiza está construido sobre la negativa a hacer eso.",
    ctaBaixar: "Descargar para Windows",
    ctaComprar: "Ver el precio",
    ctaTelas: "Ver las pantallas",
    versaoPrefixo: "Versión",
    linhaMeta:
      "Gratis para descargar · activación R$ 20 BRL, una vez · Windows 10 y 11, 64 bits",
    captura: {
      titulo: "Panel",
      descricao: "la pantalla inicial: qué está frenando esta PC, medido en el momento",
    },
  },

  numeros: [
    { valor: "677", legenda: "pruebas automatizadas, cero advertencias", fonte: "commit de la 1.7.0" },
    {
      valor: "1,2 s",
      legenda: "para abrir, desde 3,7 s en la versión anterior",
      fonte: "notas de la 1.7.0",
    },
    { valor: "byte a byte", legenda: "la precisión del deshacer", fonte: "PROGRESS.md" },
    { valor: "cero", legenda: "datos suyos que salen de su máquina", fonte: "no hay capa de red" },
  ],

  pilares: {
    rotulo: "EL PROBLEMA",
    titulo: "Todo optimizador de PC anuncia un número que usted no tiene cómo verificar.",
    itens: [
      {
        numero: "01",
        rotulo: "MEDICIÓN",
        titulo: "Mide antes y después — y admite cuando no cambió nada",
        texto:
          "Los umbrales de ruido no se inventaron: salieron de una prueba que mide la misma máquina " +
          "tres veces sin cambiar nada. Lo que varió ahí es ruido, y nunca se reporta como ganancia.",
        detalhe:
          "Dos de las seis métricas ni siquiera generan veredicto — la calibración demostró que oscilan demasiado por sí solas.",
        fonte: "README.md · noise_calibration",
      },
      {
        numero: "02",
        rotulo: "EL TIRÓN",
        titulo: "Mide el tirón, no el promedio de FPS",
        texto:
          "Nadie se queja de un promedio bajo — se queja de que el juego se traba. Un congelamiento " +
          "de 40 ms arruina la fluidez y casi no mueve un promedio de 60 cuadros por segundo.",
        /* "planificador de Windows" = el planificador de hilos del sistema.
           NO es el Programador de tareas. */
        detalhe:
          "Otimiza lee el retraso del planificador de Windows directamente, que es lo que el jugador siente.",
        fonte: "README.md · módulo jitter",
      },
      {
        numero: "03",
        rotulo: "NEGATIVA",
        titulo: "Se niega a medir con la computadora ocupada",
        texto:
          "Comparar una computadora ocupada con una descansada inventa una ganancia de decenas por " +
          "ciento. Por encima del 25% de uso de CPU, no se emite ningún veredicto.",
        detalhe: "Y la pantalla explica por qué, en vez de mostrar un número que no vale.",
        fonte: "README.md",
      },
      {
        numero: "04",
        rotulo: "HARDWARE",
        titulo: "Lee su máquina antes de ofrecer",
        texto:
          "Desactivar SysMain ayuda en SSD y estorba en disco mecánico. Apagar la compresión de " +
          "memoria ayuda con RAM de sobra y empeora con 8 GB.",
        detalhe: "Otimiza detecta el hardware y no ofrece lo que le haría mal.",
        fonte: "README.md",
      },
      {
        numero: "05",
        rotulo: "REVERSIBLE",
        titulo: "Deshacer restaura idéntico, byte a byte",
        texto:
          "Cada cambio graba el estado anterior antes de escribir. Deshacer no restaura algo " +
          "equivalente: restaura exactamente lo que había.",
        detalhe:
          "El ciclo completo se ejecutó contra el Registro de Windows y se verificó por fuera, con PowerShell.",
        fonte: "PROGRESS.md · ciclo real",
      },
      {
        numero: "06",
        rotulo: "YA OPTIMIZADO",
        titulo: "Dice cuando no hay nada que hacer",
        texto:
          "Si la configuración ya está aplicada, muestra “ya optimizado” en vez de fingir trabajo y " +
          "cobrarse el crédito por un cambio que no ocurrió.",
        detalhe: null,
        fonte: "README.md",
      },
    ],
    provaRotulo: "LA PRUEBA",
    provaTitulo: "El antes y el después, lado a lado, con el veredicto de cada métrica.",
    provaCaptura: {
      titulo: "Optimizaciones",
      descricao: "medir antes, optimizar, medir de nuevo — y el catálogo diciendo qué ya está optimizado",
    },
  },

  telas: {
    rotulo: "LAS PANTALLAS",
    titulo: "El programa entero, sin pantalla escondida.",
    itens: [
      {
        arquivo: "diagnostico.png",
        titulo: "Diagnóstico",
        descricao: "monitores, memoria instalada y los hallazgos de esta máquina",
      },
      {
        arquivo: "jogos.png",
        titulo: "Juegos",
        descricao: "la placa de video, el driver, y la configuración del juego que más mueve los FPS",
      },
      {
        arquivo: "espaco.png",
        titulo: "Espacio",
        descricao: "lo que se puede liberar — y, antes de eso, adónde fue a parar el disco",
      },
      {
        arquivo: "sistema.png",
        titulo: "Sistema",
        descricao: "lo que arranca junto con Windows, en los tres lugares donde se esconde",
      },
      {
        arquivo: "reparo.png",
        titulo: "Reparación",
        descricao: "las herramientas de Windows que devuelven archivos dañados al original",
      },
    ],
  },

  recusas: {
    rotulo: "LO QUE SE NIEGA A HACER",
    titulo: "Cinco cosas que no hace, y el motivo de cada una.",
    itens: [
      {
        titulo: "No desactiva las protecciones contra Spectre/Meltdown",
        porque: "Rendiría FPS real. También dejaría su máquina expuesta a una falla conocida.",
      },
      {
        titulo: "No toca Windows Update, ni Defender, ni el firewall",
        porque:
          "Es el ajuste preferido de los optimizadores que circulan por internet, y es lo que convierte la computadora en blanco.",
      },
      {
        titulo: "No hace “limpieza de registro”",
        porque: "No tiene ganancia medible y rompe programas instalados. Es teatro caro.",
      },
      {
        titulo: "No libera RAM a la fuerza",
        porque:
          "Deja el gráfico lindo y la computadora más lenta — Windows saca del caché lo que iba a usar.",
      },
      {
        titulo: "No escribe en el BIOS",
        porque:
          "En placas de consumo, equivocarse ahí inutiliza la placa madre. Lee, señala dónde resolverlo, y ahí se detiene.",
      },
    ],
  },

  privacidade: {
    rotulo: "PRIVACIDAD",
    titulo: "No manda nada suyo a ninguna parte, porque no sabe cómo.",
    texto:
      "Otimiza no tiene capa de red. No hay servidor adonde mandar, no hay telemetría para apagar en " +
      "las opciones. Esto no es una política de privacidad que usted tenga que creer — es algo que " +
      "el programa no puede hacer.",
    itens: [
      "El código fuente es público y puede ser leído por quien lo instala",
      "La licencia se verifica en su máquina, sin consultar ningún servidor",
      "La verificación usa Ed25519: el programa carga solo la clave que verifica, nunca la que firma",
    ],
  },

  licenca: {
    rotulo: "CÓMO FUNCIONA LA LICENCIA",
    titulo: "Una clave, una computadora — y formatear no cuesta una clave nueva.",
    itens: [
      {
        titulo: "Atada al número de serie de la placa madre",
        texto:
          "Por eso formatear Windows no invalida su clave. Sigue valiendo en la misma computadora.",
      },
      {
        titulo: "Cambiar la placa madre cambia el código",
        texto:
          "Ahí la clave deja de valer, y usted me escribe para reemitirla sin costo. Está escrito aquí " +
          "porque necesita saberlo antes de comprar, no cuando ocurra.",
      },
      {
        titulo: "De por vida, sin mensualidad",
        texto: "La compró, es suya. No hay suscripción, no hay renovación, no hay cobro recurrente.",
      },
    ],
    fluxoRotulo: "DE LA INSTALACIÓN A LA CLAVE PEGADA",
    fluxo: [
      {
        passo: "01",
        titulo: "Instálelo y ábralo",
        texto:
          "Otimiza muestra el código de esta máquina, en el formato OTZ-XXXX-XXXX-XXXX. El diagnóstico completo ya corre aquí, antes de cualquier pago.",
      },
      {
        passo: "02",
        titulo: "Pague y envíe el código",
        /* CONFIRMAR COM O DONO — el canal de entrega sigue sin definirse. Este
           texto describe solo el mecanismo, que es verdadero. */
        texto:
          "La clave se emite para ese código específico, con firma Ed25519. El canal de envío se informa en el checkout.",
      },
      {
        passo: "03",
        titulo: "Pegue la clave",
        texto:
          "La verificación ocurre dentro del programa, en su máquina. No se consulta ningún servidor, ni hay activación en línea.",
      },
    ],
    nota:
      "Antes de activar, Otimiza mide su máquina y muestra en la pantalla de compra el principal " +
      "problema que encontró en ella — medido en el momento, no texto publicitario. El programa " +
      "completo se abre con la clave.",
  },

  precos: {
    rotulo: "PRECIO",
    titulo: "Una licencia de por vida, para una computadora.",
    texto:
      "No hay plan mensual, ni plan anual, ni versión “Pro” con el mismo programa y un botón de más. " +
      "Es un producto, un precio, y es suyo.",

    planoRotulo: "LICENCIA DE POR VIDA",
    planoTitulo: "Otimiza para 1 computadora",
    planoResumo: "Pago único. Sin suscripción, sin renovación, sin cobro recurrente.",
    ctaTexto: "Comprar",

    caminhoCompra: "/es/comprar",

    formasRotulo: "FORMAS DE PAGO",
    /* Pix y Boleto son medios de pago brasilenos y conservan su nombre: no
       tienen equivalente, y renombrarlos escondería el hecho de que puede que
       no esten disponibles para un comprador fuera de Brasil.
       CONFIRMAR COM O DONO — ver notaMoeda abajo. */
    formas: ["Pix", "Tarjeta de crédito", "Boleto"],
    processadoPor: "Procesado por Mercado Pago.",
    requisito: "Windows 10 u 11, 64 bits.",

    incluiRotulo: "LO QUE NECESITA SABER ANTES DE PAGAR",
    inclui: [
      {
        titulo: "Pago único, sin suscripción",
        texto: "La licencia es de por vida. La compró, es suya — no hay renovación ni mensualidad.",
      },
      {
        titulo: "Vale para 1 computadora",
        texto:
          "La clave nace atada al número de serie de la placa madre. No abre en otra máquina, y para eso existe.",
      },
      {
        titulo: "Formatear Windows no invalida la clave",
        texto:
          "El código de la máquina viene de la placa madre, que sobrevive al formateo. Reinstalar Windows no cuesta una clave nueva.",
      },
      {
        titulo: "¿Cambió la placa madre? La reemisión es gratuita",
        texto:
          "Cambiar la placa cambia el código de la máquina y la clave deja de valer. Usted manda el código nuevo y recibe otra clave, sin costo.",
      },
      {
        titulo: "Verificación hecha en su máquina",
        texto:
          "La clave se verifica localmente, sin consultar ningún servidor. Otimiza no tiene capa de red.",
      },
      {
        titulo: "Todos los cambios son reversibles",
        texto:
          "Cada cambio graba el estado anterior antes de escribir, y el deshacer restaura idéntico, byte a byte.",
      },
    ],

    parcelamento: (parcelas, valor) => `o ${parcelas} cuotas de ${valor} sin interés con tarjeta`,

    /* CONFIRMAR COM O DONO — el precio queda en REALES (BRL) en los tres
       idiomas, porque es lo que cobra Mercado Pago. Convertirlo a pesos en
       pantalla seria mentir sobre el valor cobrado. Lo que todavia NO esta
       decidido: si el producto se va a vender fuera de Brasil, y si el checkout
       de Mercado Pago acepta tarjeta internacional (Pix y Boleto no le sirven a
       un comprador de afuera). Mientras no haya decision, el sitio dice la
       verdad sobre la moneda y nada mas. */
    sufixoMoeda: "BRL",
    notaMoeda: "Se cobra en reales brasileños (BRL) a través de Mercado Pago.",

    adicional: {
      rotulo: "OPCIONAL",
      titulo: "Licencia adicional para otra computadora",
      texto:
        "Cada computadora necesita su propia clave. Si quiere instalarlo en una segunda máquina — la del trabajo, la del hijo —, es una licencia más, también de por vida.",
      /* CONFIRMAR COM O DONO — hay descuento en la segunda licencia? Ver
         PRECO_LICENCA_ADICIONAL_BRL en src/data/precos.ts. */
      observacao: "No es obligatorio. La licencia única ya alcanza para quien tiene una computadora.",
      porComputador: "por computadora",
      comprar: "Comprar",
    },
  },

  faq: {
    rotulo: "PREGUNTAS",
    titulo: "Antes de que pague.",
    texto:
      "Las dudas que aparecen en soporte, respondidas aquí — incluso las que no favorecen la venta.",
    itens: [
      {
        pergunta: "¿Es otro optimizador más que promete FPS y no entrega nada?",
        resposta: [
          "Es contra eso que se construyó el producto. Otimiza mide la máquina antes y después de cada cambio y muestra el número — incluso cuando el número dice que no cambió nada.",
          "Los umbrales que separan ganancia de ruido no se inventaron: salieron de una prueba que mide la misma máquina tres veces sin cambiar nada. Lo que osciló ahí es ruido, y nunca se reporta como ganancia. Dos de las seis métricas ni siquiera generan veredicto, porque la calibración demostró que varían demasiado por sí solas.",
        ],
      },
      {
        pergunta: "¿Y si no mejora nada en mi computadora?",
        resposta: [
          "Lo va a decir, con número. Si la configuración ya está aplicada, la pantalla muestra “ya optimizado” en vez de fingir trabajo. Y si la computadora está ocupada — por encima del 25% de uso de CPU — no se emite ningún veredicto, porque comparar una computadora ocupada con una descansada inventa ganancia.",
          "Conviene saberlo antes de decidir: al instalar, antes de cualquier pago, Otimiza mide su máquina y muestra en la pantalla de compra el principal problema que encontró en ella — medido en el momento, no texto publicitario. El programa completo se abre con la clave.",
        ],
      },
      {
        pergunta: "¿Es seguro? ¿Esto puede romper mi Windows?",
        resposta: [
          "Cada cambio graba el estado anterior antes de escribir, y el deshacer restaura idéntico, byte a byte — no algo equivalente. Cuando Otimiza no logra leer el estado anterior, se niega a actuar: sin saber qué había antes, no hay cómo prometer el deshacer.",
          "También se niega a hacer lo que la categoría suele hacer: no desactiva las protecciones contra Spectre/Meltdown, no toca Windows Update, ni Defender, ni el firewall, no hace “limpieza de registro”, no libera RAM a la fuerza y no escribe en el BIOS.",
        ],
      },
      {
        pergunta: "¿Necesito internet para usarlo?",
        resposta: [
          "No. Otimiza no tiene capa de red: no hay activación en línea, ni verificación periódica de licencia. Una vez instalado, funciona con la máquina desconectada.",
        ],
      },
      {
        pergunta: "¿Recopilan mis datos?",
        resposta: [
          "No — y esto no es una promesa que usted tenga que creer. No existe servidor adonde mandar ni telemetría para apagar en las opciones: es algo que el programa no puede hacer.",
          "El código fuente es público y puede ser leído por quien lo instala. La verificación de la licencia usa Ed25519, y el programa carga solamente la clave que verifica la firma — nunca la que firma. No consulta ningún servidor para saber si usted puede usarlo.",
        ],
      },
      {
        pergunta: "Formateé Windows. ¿Perdí mi clave?",
        resposta: [
          "No. El código de la máquina viene del número de serie de la placa madre, y esa es la razón de la elección: sobrevive al formateo. Reinstale Windows, instale Otimiza y pegue la misma clave.",
          "Hay una excepción, y el programa se lo avisa en pantalla: en algunas máquinas el fabricante deja el número de serie de la placa en blanco. En esos casos el código se deriva del identificador de Windows, y entonces formatear cambia el código. Si eso le pasa, la reemisión también es gratuita.",
        ],
      },
      {
        pergunta: "Cambié la placa madre. ¿Y ahora?",
        resposta: [
          "La clave deja de valer, porque el código de la máquina cambió. Usted manda el código nuevo y recibe otra clave, sin costo.",
          "Esto está escrito aquí, antes de la compra, porque usted necesita saberlo ahora — y no el día en que ocurra.",
        ],
      },
      {
        pergunta: "¿Puedo usar la misma clave en dos computadoras?",
        resposta: [
          "No. Cada clave vale en una sola máquina, y es exactamente eso lo que existe para hacer. Pasarle su clave a otra persona no funciona — no abre en otra computadora.",
          "Para una segunda computadora, es una licencia adicional, también de por vida.",
        ],
      },
      {
        pergunta: "¿Funciona en Windows 10?",
        resposta: [
          "Sí. Windows 10 u 11, 64 bits. No hay versión para macOS ni Linux: lo que Otimiza hace depende del Registro de Windows y de los servicios de Windows.",
        ],
      },
      {
        pergunta: "¿Por qué Windows dice “editor desconocido” cuando lo instalo?",
        resposta: [
          "Porque el instalador todavía no tiene firma digital. SmartScreen muestra “Windows protegió su PC” en la primera ejecución; basta con hacer clic en “Más información” y después en “Ejecutar de todas formas”.",
          "Esto está escrito en las notas de cada versión en vez de escondido. El certificado de firma de código exige compra y verificación de identidad, y todavía no se hizo.",
        ],
      },
      {
        pergunta: "¿Cómo recibo la clave después de pagar?",
        /* CONFIRMAR COM O DONO — canal de entrega sin definir; el texto describe
           solo el mecanismo, que es verdadero. */
        confirmar: true,
        resposta: [
          "La clave se emite para el código de su máquina, así que el paso es: instalar Otimiza, copiar el código que aparece en pantalla — en el formato OTZ-XXXX-XXXX-XXXX — y enviarlo junto con el pago. La clave vuelve lista para pegar en el campo.",
          "Las instrucciones de envío y el plazo aparecen en el checkout.",
        ],
      },
      {
        pergunta: "¿Hay reembolso?",
        /* CONFIRMAR COM O DONO — la politica de reembolso es decision comercial
           y juridica. NO inventar plazo, condicion ni garantia aqui, en ningun
           idioma. */
        confirmar: true,
        resposta: [
          "La política de reembolso se informará en la página de compra, antes de la confirmación del pago.",
        ],
      },
    ],
  },

  rodape: {
    links: [
      { href: "#pilares", texto: "Cómo funciona" },
      { href: "#telas", texto: "Las pantallas" },
      { href: "#recusas", texto: "Lo que no hace" },
      { href: "#licenca", texto: "La licencia" },
      { href: "#privacidade", texto: "Privacidad" },
      { href: "#precos", texto: "Precio" },
    ],
    /* NO afirmar aqui que el codigo fuente es publico o auditable: esa
       afirmacion fue retirada a proposito. */
    direitos: "© {ano} Otimiza. Código abierto a la lectura, no al uso.",
  },
};
