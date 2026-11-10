// Os ícones da interface, desenhados para o Otimiza: traço de 1,75 num quadro de 24×24, pontas e cantos redondos,
// sem preenchimento. Uma família só — nada de misturar com ícone cheio de outro conjunto. Ícone é para achar um
// lugar (trilho, cabeçalho) ou um controle que não cabe em palavra (busca, fechar); lista e botão com texto não
// levam ícone.

export const ICONES = {
  // Áreas
  casa: `<path d="M4 10.4 12 4l8 6.4V19a1 1 0 0 1-1 1h-4.5v-5.5h-5V20H5a1 1 0 0 1-1-1Z"/>`,
  raio: `<path d="M13.2 3 5.5 13.2h5.8L10.6 21l7.9-10.2h-5.8Z"/>`,
  controle: `<path d="M7.6 7.5h8.8a4.6 4.6 0 0 1 4.6 4.6v2.6a2.8 2.8 0 0 1-5.1 1.6L14.6 14.6H9.4l-1.3 1.7A2.8 2.8 0 0 1 3 14.7v-2.6a4.6 4.6 0 0 1 4.6-4.6Z"/><path d="M8.2 9.8v3.4M6.5 11.5h3.4"/><path d="M15.4 10.6h.01M17.2 12.4h.01" stroke-width="2.4"/>`,
  wifi: `<path d="M2.8 9.2a13.5 13.5 0 0 1 18.4 0M5.8 12.5a9 9 0 0 1 12.4 0M8.9 15.7a4.6 4.6 0 0 1 6.2 0"/><path d="M12 19.2h.01" stroke-width="2.6"/>`,
  cronometro: `<circle cx="12" cy="13.5" r="7.5"/><path d="M12 10v3.5l2.3 1.5M10 3h4M18.3 6.6l1.3-1.3"/>`,
  monitor: `<rect x="3" y="4.5" width="18" height="11.5" rx="1.8"/><path d="M12 16v3.5M8.5 19.5h7"/>`,
  relogio: `<path d="M4.2 12.8A7.9 7.9 0 1 0 6.5 6.4L4 8.9"/><path d="M4 4.6v4.3h4.3M12 8.2V12l2.8 1.8"/>`,
  ajustes: `<path d="M4 7h9.5M17.5 7H20M4 17h2.5M10.5 17H20"/><circle cx="15.5" cy="7" r="2"/><circle cx="8.5" cy="17" r="2"/>`,
  energia: `<path d="M4.6 17.5a8.5 8.5 0 1 1 14.8 0"/><path d="m12 14 3.6-4.2"/><path d="M12 14h.01" stroke-width="2.8"/>`,
  chip: `<rect x="6.5" y="6.5" width="11" height="11" rx="2"/><path d="M10 10h4v4h-4zM10 3.5v3M14 3.5v3M10 17.5v3M14 17.5v3M3.5 10h3M3.5 14h3M17.5 10h3M17.5 14h3"/>`,
  grafico: `<path d="M4 20h16M7.5 16.5v-4M12 16.5v-9M16.5 16.5v-6"/>`,
  mira: `<circle cx="12" cy="12" r="7.5"/><path d="M12 2.8v5M12 16.2v5M2.8 12h5M16.2 12h5"/>`,
  gota: `<path d="M12 3.6c3.1 3.6 5.6 6.6 5.6 9.7a5.6 5.6 0 0 1-11.2 0c0-3.1 2.5-6.1 5.6-9.7Z"/><path d="M9.4 14a2.7 2.7 0 0 0 2 2.4"/>`,
  grade: `<rect x="4.5" y="4.5" width="6" height="6" rx="1.6"/><rect x="13.5" y="4.5" width="6" height="6" rx="1.6"/><rect x="4.5" y="13.5" width="6" height="6" rx="1.6"/><rect x="13.5" y="13.5" width="6" height="6" rx="1.6"/>`,

  // Controles
  busca: `<circle cx="10.8" cy="10.8" r="6.3"/><path d="m15.5 15.5 4.5 4.5"/>`,
  mais: `<path d="M12 5.5v13M5.5 12h13"/>`,
  fechar: `<path d="m6.5 6.5 11 11M17.5 6.5l-11 11"/>`,
  seta_baixo: `<path d="m6.5 9.5 5.5 5.5 5.5-5.5"/>`,
  seta_cima: `<path d="m6.5 14.5 5.5-5.5 5.5 5.5"/>`,
  seta_direita: `<path d="m9.5 6.5 5.5 5.5-5.5 5.5"/>`,
  sino: `<path d="M6.3 16.5V11a5.7 5.7 0 0 1 11.4 0v5.5l1.5 1.8H4.8Z"/><path d="M10 20.6h4"/>`,
  sol: `<circle cx="12" cy="12" r="3.8"/><path d="M12 3v1.8M12 19.2V21M3 12h1.8M19.2 12H21M5.6 5.6l1.3 1.3M17.1 17.1l1.3 1.3M5.6 18.4l1.3-1.3M17.1 6.9l1.3-1.3"/>`,
  lua: `<path d="M19.5 14.6A7.6 7.6 0 0 1 9.4 4.5a7.8 7.8 0 1 0 10.1 10.1Z"/>`,
  desfazer: `<path d="M8.5 13.5 4 9l4.5-4.5"/><path d="M4 9h10a5.5 5.5 0 0 1 0 11h-3"/>`,
} as const;

export type NomeDoIcone = keyof typeof ICONES;
