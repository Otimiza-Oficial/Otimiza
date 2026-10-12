// Laboratório E6: onde o processador gastou o tempo durante o jogo, a partir de uma gravação do WPR
// (scripts/laboratorio/gravar-fivem-wpr.ps1). Responde, sem símbolos (nomes de função), por processo, thread, módulo
// e driver:
//   1. quanto do processador cada programa usou (o jogo e quem disputa com ele);
//   2. a thread principal do jogo: quanto ela ocupa de um núcleo (perto de 100% = preso no processador) e em quê;
//   3. quanto tempo ela ficou PRONTA esperando núcleo livre (disputa com outros programas);
//   4. interrupções de driver (DPC/ISR): quem e quanto, e a mais longa (acima de 1 ms vira tranco);
//   5. frequência do processador e núcleos estacionados durante o jogo.
//
//   dotnet run -c Release -- <arquivo.etl> [nome do processo do jogo, padrão: GTAProcess]
using Microsoft.Windows.EventTracing;
using Microsoft.Windows.EventTracing.Cpu;
using Microsoft.Windows.EventTracing.Power;
using Microsoft.Windows.EventTracing.Processes;
using Microsoft.Windows.EventTracing.Symbols;

if (args.Length < 1) { Console.WriteLine("uso: analise-wpr <arquivo.etl> [parte do nome do processo do jogo]"); return 1; }
string jogoNome = args.Length > 1 ? args[1] : "GTAProcess";

using var trace = TraceProcessor.Create(args[0], new TraceProcessorSettings { AllowLostEvents = true });
var pAmostras = trace.UseCpuSamplingData();
var pAgenda = trace.UseCpuSchedulingData();
var pInterr = trace.UseInterruptHandlingData();
var pFreq = trace.UseProcessorFrequencyData();
var pEstac = trace.UseProcessorParkingData();
// `--simbolos` (terceiro argumento): baixa os símbolos públicos da Microsoft (só do processo do jogo) para dar nome às
// funções do Windows que a thread principal executa. Cache em laboratorio/ferramentas/simbolos (fora do GitHub).
bool comSimbolos = args.Contains("--simbolos");
var pSimbolos = comSimbolos ? trace.UseSymbols() : null;
trace.Process();
if (comSimbolos)
{
    string cache = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "ferramentas", "simbolos"));
    Directory.CreateDirectory(cache);
    var processoDoJogo = pAmostras.Result.Samples.Select(s => s.Process?.ImageName).FirstOrDefault(n => n != null && n.Contains(jogoNome, StringComparison.OrdinalIgnoreCase));
    Console.WriteLine($"Baixando símbolos da Microsoft para {processoDoJogo} (cache em {cache})…");
    pSimbolos.Result.LoadSymbolsForConsoleAsync(
        new Microsoft.Windows.EventTracing.Symbols.SymCachePath(Path.Combine(cache, "symcache")),
        new Microsoft.Windows.EventTracing.Symbols.SymbolPath($"srv*{cache}*https://msdl.microsoft.com/download/symbols"),
        new[] { processoDoJogo }, null).GetAwaiter().GetResult();
}

static double Ms(decimal v) => (double)v;
var amostras = pAmostras.Result.Samples;
if (amostras.Count == 0) { Console.WriteLine("A gravação não tem amostras de processador."); return 1; }
double inicio = Ms(amostras.Min(s => s.Timestamp.TotalMilliseconds)), fim = Ms(amostras.Max(s => s.Timestamp.TotalMilliseconds));
double duracao = Math.Max(fim - inicio, 1);
int nucleos = amostras.Select(s => s.Processor).Distinct().Count();
double capacidade = duracao * nucleos;
Console.WriteLine($"Gravação: {duracao / 1000:F1} s, {nucleos} processadores lógicos, {amostras.Count:N0} amostras.");

string NomeDe(ICpuSample s)
{
    if (s.IsExecutingDeferredProcedureCall == true) return $"[DPC] {s.Image?.FileName ?? "?"}";
    if (s.IsExecutingInterruptServicingRoutine == true) return $"[ISR] {s.Image?.FileName ?? "?"}";
    return s.Process?.ImageName ?? "?";
}
bool EDoJogo(IProcess p) => p?.ImageName != null && p.ImageName.Contains(jogoNome, StringComparison.OrdinalIgnoreCase);

// 1. Quem usou o processador.
Console.WriteLine("\n1. QUEM USOU O PROCESSADOR (% da capacidade total; entre parênteses, em núcleos inteiros)");
foreach (var g in amostras.GroupBy(NomeDe).Select(g => (nome: g.Key, ms: g.Sum(s => Ms(s.Weight.TotalMilliseconds)))).OrderByDescending(x => x.ms).Take(14))
    Console.WriteLine($"   {g.nome,-42} {100 * g.ms / capacidade,5:F1}%  ({g.ms / duracao:F2} núcleo)");
double ocioso = amostras.Where(s => s.Process?.ImageName == "Idle").Sum(s => Ms(s.Weight.TotalMilliseconds));

// 2. As threads do jogo.
var doJogo = amostras.Where(s => EDoJogo(s.Process) && s.IsExecutingDeferredProcedureCall != true && s.IsExecutingInterruptServicingRoutine != true).ToList();
if (doJogo.Count == 0) { Console.WriteLine($"\nNenhuma amostra do jogo (\"{jogoNome}\")."); return 0; }
Console.WriteLine($"\n2. THREADS DO JOGO ({doJogo[0].Process.ImageName}) — % de UM núcleo");
var threads = doJogo.GroupBy(s => s.Thread?.Id ?? -1)
    .Select(g => (id: g.Key, nome: g.First().Thread?.Name, ms: g.Sum(s => Ms(s.Weight.TotalMilliseconds))))
    .OrderByDescending(x => x.ms).ToList();
foreach (var t in threads.Take(8))
    Console.WriteLine($"   thread {t.id,6} {(string.IsNullOrWhiteSpace(t.nome) ? "" : t.nome),-28} {100 * t.ms / duracao,5:F1}%");
var principal = threads[0];
Console.WriteLine($"   jogo inteiro: {doJogo.Sum(s => Ms(s.Weight.TotalMilliseconds)) / duracao:F2} núcleo de {nucleos}");

Console.WriteLine($"\n   Em que a thread mais ocupada ({principal.id}) gastou o tempo (módulo):");
foreach (var m in doJogo.Where(s => s.Thread?.Id == principal.id).GroupBy(s => s.Image?.FileName ?? "(sem módulo)")
             .Select(g => (nome: g.Key, ms: g.Sum(s => Ms(s.Weight.TotalMilliseconds)))).OrderByDescending(x => x.ms).Take(10))
    Console.WriteLine($"      {m.nome,-36} {100 * m.ms / principal.ms,5:F1}%");

if (comSimbolos)
{
    // O que o núcleo do Windows faz DENTRO da thread principal, e quem do jogo pediu.
    var noKernel = doJogo.Where(s => s.Thread?.Id == principal.id && string.Equals(s.Image?.FileName, "ntoskrnl.exe", StringComparison.OrdinalIgnoreCase)).ToList();
    double msKernel = noKernel.Sum(s => Ms(s.Weight.TotalMilliseconds));
    Console.WriteLine($"\n   No núcleo do Windows (ntoskrnl), {msKernel:F0} ms da thread principal. Funções:");
    foreach (var f in noKernel.GroupBy(s => s.TopStackFrame.Symbol?.FunctionName ?? "(sem símbolo)")
                 .Select(g => (nome: g.Key, ms: g.Sum(s => Ms(s.Weight.TotalMilliseconds)))).OrderByDescending(x => x.ms).Take(15))
        Console.WriteLine($"      {f.nome,-48} {100 * f.ms / msKernel,5:F1}%");
    // A chamada do sistema (a função do ntdll) e o módulo do jogo que a fez: o primeiro quadro da pilha fora do Windows.
    string[] doWindows = { "ntoskrnl.exe", "ntdll.dll", "win32kfull.sys", "win32kbase.sys", "win32u.dll", "kernelbase.dll", "kernel32.dll", "hal.dll" };
    var chamadas = new Dictionary<string, double>();
    foreach (var s in noKernel)
    {
        var quadros = s.Stack?.Frames;
        if (quadros == null) continue;
        string sistema = null, quem = null;
        foreach (var q in quadros)
        {
            string img = q.Image?.FileName ?? "?";
            if (sistema == null && string.Equals(img, "ntdll.dll", StringComparison.OrdinalIgnoreCase)) sistema = q.Symbol?.FunctionName;
            if (!doWindows.Contains(img, StringComparer.OrdinalIgnoreCase)) { quem = img; break; }
        }
        string chave = $"{sistema ?? "?"} ← {quem ?? "?"}";
        chamadas[chave] = chamadas.GetValueOrDefault(chave) + Ms(s.Weight.TotalMilliseconds);
    }
    Console.WriteLine("   Chamada do sistema ← quem do jogo pediu:");
    foreach (var c in chamadas.OrderByDescending(c => c.Value).Take(15))
        Console.WriteLine($"      {c.Key,-60} {100 * c.Value / msKernel,5:F1}%");
}

// 2b. O núcleo físico dividido: o i3 tem 2 threads por núcleo (no Windows, os processadores lógicos 2k e 2k+1). Quando
// a thread principal roda e o "irmão" dela está ocupado com outro programa, ela anda mais devagar sem aparecer como
// esperando. Amostras de 1 ms: em cada amostra da principal, o que rodava no irmão no mesmo milissegundo.
var porProcessador = new Dictionary<(int, long), string>();
double totalPeso = amostras.Sum(s => Ms(s.Weight.TotalMilliseconds));
foreach (var s in amostras)
    porProcessador[(s.Processor, (long)Ms(s.Timestamp.TotalMilliseconds))] =
        s.Process?.ImageName == "Idle" ? "(livre)" : EDoJogo(s.Process) && s.IsExecutingDeferredProcedureCall != true ? "o próprio jogo" : NomeDe(s);
var irmaos = new Dictionary<string, int>();
int daPrincipal = 0;
foreach (var s in doJogo.Where(s => s.Thread?.Id == principal.id))
{
    daPrincipal++;
    var chave = (s.Processor ^ 1, (long)Ms(s.Timestamp.TotalMilliseconds));
    string n = porProcessador.TryGetValue(chave, out var v) ? v : "(sem amostra)";
    irmaos[n] = irmaos.GetValueOrDefault(n) + 1;
}
Console.WriteLine($"\n   Amostras somadas: {100 * totalPeso / capacidade:F0}% da capacidade (o resto não foi amostrado).");
Console.WriteLine($"   Com quem a thread principal dividiu o núcleo físico ({daPrincipal:N0} amostras dela):");
foreach (var i in irmaos.OrderByDescending(i => i.Value).Take(8))
    Console.WriteLine($"      {i.Key,-36} {100.0 * i.Value / Math.Max(daPrincipal, 1),5:F1}%");

// 3. Esperando núcleo livre (pronta, sem rodar) e esperando algo (bloqueada).
var atividades = pAgenda.Result.ThreadActivity.Where(a => a.Thread?.Id == principal.id && EDoJogo(a.Process)).ToList();
double pronta = atividades.Sum(a => a.ReadyDuration.HasValue ? Ms(a.ReadyDuration.Value.TotalMilliseconds) : 0);
double esperando = atividades.Sum(a => a.WaitingDuration.HasValue ? Ms(a.WaitingDuration.Value.TotalMilliseconds) : 0);
double rodando = atividades.Sum(a => Ms(a.Duration.TotalMilliseconds));
Console.WriteLine($"\n3. A THREAD {principal.id} NO TEMPO TODO: rodando {100 * rodando / duracao:F1}% · pronta esperando núcleo {100 * pronta / duracao:F1}% · bloqueada esperando algo {100 * esperando / duracao:F1}%");
var quemTomou = atividades.Where(a => a.ReadyDuration.HasValue && a.PreviousActivity != null)
    .GroupBy(a => a.ReadyingProcess?.ImageName ?? "?").Select(g => (nome: g.Key, n: g.Count())).OrderByDescending(x => x.n).Take(6);
Console.WriteLine("   Quem acordou a thread (quem ela esperava): " + string.Join(", ", quemTomou.Select(q => $"{q.nome} ({q.n})")));
var fatias = pAgenda.Result.CpuTimeSlices;
// Quem estava no núcleo enquanto a thread principal esperava pronta: fatias de outros processos que começaram
// durante a espera dela, no mesmo processador em que ela entrou depois.
var ladroes = new Dictionary<string, double>();
foreach (var a in atividades.Where(a => a.ReadyDuration.HasValue && Ms(a.ReadyDuration.Value.TotalMilliseconds) > 0.05))
{
    double fimEspera = Ms(a.StartTime.TotalMilliseconds), iniEspera = fimEspera - Ms(a.ReadyDuration.Value.TotalMilliseconds);
    foreach (var f in fatias)
    {
        if (f.Processor != a.Processor) continue;
        double fi = Ms(f.StartTime.TotalMilliseconds), ff = Ms(f.StopTime.TotalMilliseconds);
        if (ff <= iniEspera || fi >= fimEspera || f.Thread?.Id == principal.id) continue;
        string n = f.Process?.ImageName ?? "?";
        ladroes[n] = ladroes.GetValueOrDefault(n) + Math.Min(ff, fimEspera) - Math.Max(fi, iniEspera);
    }
}
Console.WriteLine("   Quem ocupava o núcleo enquanto ela esperava: " + string.Join(", ", ladroes.OrderByDescending(l => l.Value).Take(8).Select(l => $"{l.Key} {l.Value:F0} ms")));

// 4. Interrupções de driver.
var interr = pInterr.Result.Activity;
Console.WriteLine($"\n4. INTERRUPÇÕES DE DRIVER (DPC/ISR): {interr.Count:N0} no total");
foreach (var g in interr.GroupBy(i => $"{i.Interval.Type} {i.Interval.HandlerImage?.FileName ?? "?"}")
             .Select(g => (nome: g.Key, ms: g.Sum(i => Ms(i.Duration.TotalMilliseconds)), maior: g.Max(i => Ms(i.Duration.TotalMilliseconds)), n: g.Count()))
             .OrderByDescending(x => x.ms).Take(10))
    Console.WriteLine($"   {g.nome,-40} {g.ms,8:F1} ms ({100 * g.ms / capacidade:F2}%) · {g.n,6} vezes · a mais longa {g.maior:F3} ms");
var longas = interr.Where(i => Ms(i.Duration.TotalMilliseconds) > 1).ToList();
Console.WriteLine($"   Acima de 1 ms (podem virar tranco): {longas.Count}" + (longas.Count > 0 ? " — " + string.Join(", ", longas.GroupBy(i => i.Interval.HandlerImage?.FileName ?? "?").Select(g => $"{g.Key} ×{g.Count()}")) : ""));

// 5. Frequência e núcleos estacionados.
var freqs = pFreq.Result.Intervals;
if (freqs.Count > 0)
{
    double soma = 0, peso = 0, menor = double.MaxValue;
    foreach (var f in freqs)
    {
        double d = Ms((f.StopTime - f.StartTime).TotalMilliseconds);
        if (d <= 0) continue;
        soma += f.AverageFrequencyAsPercentOfBase * d; peso += d; menor = Math.Min(menor, f.AverageFrequencyAsPercentOfBase);
    }
    var mhz = freqs.Where(f => f.AverageFrequency.HasValue).Select(f => (double)f.AverageFrequency.Value.TotalMegahertz).DefaultIfEmpty(0).Average();
    Console.WriteLine($"\n5. FREQUÊNCIA: média {(peso > 0 ? soma / peso : 0):F0}% da base (≈{mhz:F0} MHz), menor intervalo {menor:F0}% da base");
}
var estac = pEstac.Result.Intervals;
if (estac.Count > 0)
{
    var porNucleo = estac.GroupBy(e => e.Processor).Select(g => (p: g.Key, pct: 100 * g.Where(e => e.IsParked).Sum(e => Ms((e.StopTime - e.StartTime).TotalMilliseconds)) / duracao)).OrderBy(x => x.p);
    Console.WriteLine("   Núcleos estacionados (% do tempo): " + string.Join("  ", porNucleo.Select(x => $"#{x.p} {x.pct:F0}%")));
}
else Console.WriteLine("   Núcleos estacionados: nenhum evento (nenhum núcleo estacionou, ou a gravação não pegou).");
return 0;
