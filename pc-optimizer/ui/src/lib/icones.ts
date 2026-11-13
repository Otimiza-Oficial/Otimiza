// Os ícones da interface, desenhados para o Otimiza: traço de 1,75 num quadro de 24×24, pontas e cantos redondos, e
// o corpo de cada um (`class="c"`) com um véu da mesma cor, para o ícone ter peso sem virar bloco cheio. Uma família
// só: nada de misturar com ícone de outro conjunto. Ícone é para achar um lugar (trilho) ou um controle que não cabe
// em palavra (busca, fechar); lista e botão com texto não levam ícone.

export const ICONES = {
  // Áreas
  casa: `<path class="c" d="M4.5 10.3 12 4.5l7.5 5.8v8.2a1.5 1.5 0 0 1-1.5 1.5H6a1.5 1.5 0 0 1-1.5-1.5Z"/><path d="M9.8 20v-4a2.2 2.2 0 0 1 4.4 0v4"/>`,
  raio: `<path class="c" d="M13.4 3.5 5.8 13.1h5.6l-1 7.4 7.8-9.7h-5.6Z"/>`,
  controle: `<path class="c" d="M8 7h8a5 5 0 0 1 5 5v2.6a2.9 2.9 0 0 1-5.3 1.6L14.6 15H9.4l-1.1 1.2A2.9 2.9 0 0 1 3 14.6V12a5 5 0 0 1 5-5Z"/><path d="M8 9.9v4.2M5.9 12h4.2"/><circle cx="15.4" cy="10.9" r="1.05" fill="currentColor" stroke="none"/><circle cx="17.6" cy="13.1" r="1.05" fill="currentColor" stroke="none"/>`,
  wifi: `<path class="c" d="M12 18.6 4.8 10.9a10.6 10.6 0 0 1 14.4 0Z" stroke="none"/><path d="M3.6 9.6a12 12 0 0 1 16.8 0M6.6 12.7a7.7 7.7 0 0 1 10.8 0M9.6 15.8a3.5 3.5 0 0 1 4.8 0"/><circle cx="12" cy="19" r="1.2" fill="currentColor" stroke="none"/>`,
  cronometro: `<circle class="c" cx="12" cy="13.6" r="7"/><path d="M12 13.6v-3.4M9.8 3.5h4.4M12 3.5v3.1M17.6 7.9l1.4-1.4"/>`,
  monitor: `<rect class="c" x="3" y="4.5" width="18" height="12" rx="2.4"/><path d="M12 16.5v3.5M8.6 20h6.8"/>`,
  relogio: `<circle class="c" cx="12" cy="12" r="7.9" stroke="none"/><path d="M4.1 12.6A8 8 0 1 0 6.4 6.3"/><path d="M4.2 4.3v4h4"/><path d="M12 8.3v3.9l2.6 1.6"/>`,
  ajustes: `<path d="M4 8h7.6M16.4 8H20M4 16h3.6M12.4 16H20"/><circle class="c" cx="14" cy="8" r="2.4"/><circle class="c" cx="10" cy="16" r="2.4"/>`,
  energia: `<path class="c" d="M4.6 17.5a8.5 8.5 0 1 1 14.8 0Z" stroke="none"/><path d="M4.6 17.5a8.5 8.5 0 1 1 14.8 0"/><path d="m12 14 3.6-4.2"/><circle cx="12" cy="14" r="1.3" fill="currentColor" stroke="none"/>`,
  chip: `<rect class="c" x="6.5" y="6.5" width="11" height="11" rx="2.2"/><path d="M10 3.5v3M14 3.5v3M10 17.5v3M14 17.5v3M3.5 10h3M3.5 14h3M17.5 10h3M17.5 14h3"/><rect x="10" y="10" width="4" height="4" rx=".8" fill="currentColor" stroke="none"/>`,
  grafico: `<path d="M4 20h16"/><rect class="c" x="5.6" y="12" width="3.6" height="5" rx="1"/><rect class="c" x="10.2" y="6.5" width="3.6" height="10.5" rx="1"/><rect class="c" x="14.8" y="9.5" width="3.6" height="7.5" rx="1"/>`,
  mira: `<circle class="c" cx="12" cy="12" r="7.5"/><path d="M12 2.8v5M12 16.2v5M2.8 12h5M16.2 12h5"/><circle cx="12" cy="12" r="1.2" fill="currentColor" stroke="none"/>`,
  gota: `<path class="c" d="M12 3.6c3.1 3.6 5.6 6.6 5.6 9.7a5.6 5.6 0 0 1-11.2 0c0-3.1 2.5-6.1 5.6-9.7Z"/><path d="M9.4 14a2.7 2.7 0 0 0 2 2.4"/>`,
  grade: `<rect class="c" x="4" y="4" width="6.6" height="6.6" rx="2"/><rect class="c" x="13.4" y="4" width="6.6" height="6.6" rx="2"/><rect class="c" x="4" y="13.4" width="6.6" height="6.6" rx="2"/><rect class="c" x="13.4" y="13.4" width="6.6" height="6.6" rx="2"/>`,

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
