<script lang="ts">
  // O logotipo horizontal da marca v2: o Oti é o "O" e o resto da palavra é Geist 700 já em contornos (entrega
  // `svg/horizontal-*.svg`). Nunca se digita "Otimiza" com a fonte no lugar dele. A cor segue o tema: no escuro o Oti
  // branco e a palavra branca; no claro o negativo, tudo carvão.
  let { altura = 20 }: { altura?: number } = $props();
  const id = `logo${Math.random().toString(36).slice(2, 8)}`;
</script>

<svg class="logo" viewBox="0 -726 3683 742" height={altura} width={(altura * 3683) / 742} role="img" aria-label="Otimiza">
  <defs><clipPath id="{id}-o"><circle cx="367" cy="-357" r="363.33" /></clipPath></defs>
  <circle class="disco" cx="367" cy="-357" r="367" />
  <g clip-path="url(#{id}-o)">
    <rect class="olho" x="519.30" y="-580.87" width="62.39" height="190.84" rx="31.20" transform="rotate(-32 550.50 -485.45)" />
    <rect class="olho" x="633.08" y="-650.60" width="40.37" height="190.84" rx="20.18" transform="rotate(-32 653.26 -555.18)" />
  </g>
  <path
    class="palavra"
    d="M1066 0Q983 0 944-38.5Q905-77 905-161V-662H1055V-179Q1055-140 1071.5-126Q1088-112 1121-112H1196V0ZM819-424V-536H1196V-424ZM1250 0V-536H1400V0ZM1247-602V-722H1402V-602ZM1509 0V-536H1644L1649-407 1635-413Q1646-456 1668.5-486Q1691-516 1723-532Q1755-548 1793-548Q1860-548 1901.5-509Q1943-470 1955-404L1936-403Q1945-450 1967.5-482.5Q1990-515 2023-531.5Q2056-548 2097-548Q2153-548 2193-525Q2233-502 2255-456.5Q2277-411 2277-345V0H2127V-303Q2127-366 2108-397.5Q2089-429 2046-429Q2019-429 2000-414Q1981-399 1970.5-370.5Q1960-342 1960-301V0H1825V-301Q1825-363 1807-396Q1789-429 1745-429Q1718-429 1698.5-414Q1679-399 1669-370Q1659-341 1659-301V0ZM2375 0V-536H2525V0ZM2372-602V-722H2527V-602ZM2602 0V-118L2897-424H2614V-536H3077V-418L2777-112H3085V0ZM3314 12Q3231 12 3178.5-26.5Q3126-65 3126-134Q3126-205 3170.5-243.5Q3215-282 3302-300L3475-334Q3475-386 3451.5-412.5Q3428-439 3384-439Q3341-439 3317-419.5Q3293-400 3286-362L3135-369Q3150-457 3214.5-502.5Q3279-548 3384-548Q3504-548 3564.5-491Q3625-434 3625-324V-141Q3625-118 3632.5-110.5Q3640-103 3655-103H3670V0Q3663 2 3647.5 4Q3632 6 3617 6Q3585 6 3557.5-5Q3530-16 3513-42.5Q3496-69 3496-118L3509-108Q3499-72 3473.5-45Q3448-18 3407.5-3Q3367 12 3314 12ZM3350-91Q3387-91 3415-105.5Q3443-120 3459-148Q3475-176 3475-214V-240L3353-214Q3317-207 3298.5-190.5Q3280-174 3280-148Q3280-121 3298-106Q3316-91 3350-91Z"
  />
</svg>

<style>
  .logo {
    display: block;
    flex: none;
  }
  .disco {
    fill: var(--oti-disco-tema);
  }
  .olho {
    fill: var(--oti-olho-tema);
  }
  .palavra {
    fill: var(--tinta);
  }
</style>
