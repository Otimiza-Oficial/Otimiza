// Laboratório E3: a MESMA bancada de captura de src-tauri/examples/bancada_captura.rs (método dxgi), em C#.
// Sem pacote de terceiros: o DirectX é chamado pela posição de cada função na vtable (conferida no cabeçalho do
// Windows SDK 10.0.26100). Imprime a mesma linha de resultado das outras linguagens, mais as coletas do coletor de lixo.
//
//   dotnet publish -c Release -r win-x64 -o bin\publicado
//   bin\publicado\bancada_captura.exe <segundos> <arquivo do marcador do jogo>
using System.Diagnostics;
using System.Globalization;
using System.Runtime.InteropServices;

unsafe
{
    double segundos = args.Length > 0 ? double.Parse(args[0], CultureInfo.InvariantCulture) : 10.0;
    string arquivo = args.Length > 1 ? args[1] : null;

    Nativo.SetProcessDpiAwarenessContext((IntPtr)(-4)); // PER_MONITOR_AWARE_V2
    Nativo.timeBeginPeriod(1);
    Nativo.QueryPerformanceFrequency(out long freq);

    IntPtr hwnd = Nativo.FindWindowW("JogoDeTeste", null);
    if (hwnd == IntPtr.Zero) { Console.WriteLine("abra o jogo de teste antes"); return 1; }
    var origem = new Ponto();
    Nativo.ClientToScreen(hwnd, ref origem);
    IntPtr monitor = Nativo.MonitorFromWindow(hwnd, 2); // MONITOR_DEFAULTTONEAREST

    // A placa e a saída do monitor do jogo.
    Guid iidFabrica = new("770aae78-f26f-4dba-a829-253c83d1b387"), iidSaida5 = new("80a07424-ab52-42eb-833c-0c42fd282d98");
    Checar(Nativo.CreateDXGIFactory1(ref iidFabrica, out IntPtr fabrica), "CreateDXGIFactory1");
    IntPtr adaptador = IntPtr.Zero, saida5 = IntPtr.Zero;
    Ret area = default;
    for (uint i = 0; saida5 == IntPtr.Zero; i++)
    {
        IntPtr a;
        if (((delegate* unmanaged[Stdcall]<IntPtr, uint, IntPtr*, int>)Vt(fabrica)[12])(fabrica, i, &a) != 0) break; // EnumAdapters1
        for (uint j = 0; ; j++)
        {
            IntPtr o;
            if (((delegate* unmanaged[Stdcall]<IntPtr, uint, IntPtr*, int>)Vt(a)[7])(a, j, &o) != 0) break; // EnumOutputs
            DescDaSaida d;
            ((delegate* unmanaged[Stdcall]<IntPtr, DescDaSaida*, int>)Vt(o)[7])(o, &d); // GetDesc
            if (d.Monitor == monitor)
            {
                IntPtr s5;
                Checar(((delegate* unmanaged[Stdcall]<IntPtr, Guid*, IntPtr*, int>)Vt(o)[0])(o, &iidSaida5, &s5), "IDXGIOutput5");
                saida5 = s5; area = d.Area; adaptador = a;
            }
            Liberar(o);
            if (saida5 != IntPtr.Zero) break;
        }
        if (saida5 == IntPtr.Zero) Liberar(a);
    }
    if (saida5 == IntPtr.Zero) { Console.WriteLine("o monitor do jogo não apareceu no DXGI"); return 1; }

    int* niveis = stackalloc int[] { 0xb100, 0xb000 };
    Checar(Nativo.D3D11CreateDevice(adaptador, 0 /* UNKNOWN */, IntPtr.Zero, 0x20 /* BGRA */, niveis, 2, 7, out IntPtr device, IntPtr.Zero, out IntPtr ctx), "D3D11CreateDevice");

    uint* formatos = stackalloc uint[] { 10 /* R16G16B16A16_FLOAT */, 87 /* B8G8R8A8_UNORM */ };
    IntPtr dup;
    Checar(((delegate* unmanaged[Stdcall]<IntPtr, IntPtr, uint, uint, uint*, IntPtr*, int>)Vt(saida5)[26])(saida5, device, 0, 2, formatos, &dup), "DuplicateOutput1");

    uint x = (uint)(origem.X - area.Esquerda), y = (uint)(origem.Y - area.Topo);
    Guid iidTextura = new("6f15aaf2-d208-4e89-9ab4-489535d34f9c");
    var copias = new Dictionary<uint, IntPtr>();

    bool LerNumero(IntPtr textura, out uint n)
    {
        n = 0;
        DescDaTextura d;
        ((delegate* unmanaged[Stdcall]<IntPtr, DescDaTextura*, void>)Vt(textura)[10])(textura, &d);
        if (x + 160 > d.Largura || y + 8 > d.Altura) return false;
        if (!copias.TryGetValue(d.Formato, out IntPtr copia))
        {
            var c = new DescDaTextura { Largura = 160, Altura = 8, Mips = 1, Camadas = 1, Formato = d.Formato, Amostras = 1, Uso = 3 /* STAGING */, AcessoCpu = 0x20000 /* READ */ };
            Checar(((delegate* unmanaged[Stdcall]<IntPtr, DescDaTextura*, IntPtr, IntPtr*, int>)Vt(device)[5])(device, &c, IntPtr.Zero, &copia), "CreateTexture2D");
            copias[d.Formato] = copia;
        }
        var caixa = new Caixa { Esquerda = x, Topo = y, Frente = 0, Direita = x + 160, Baixo = y + 8, Tras = 1 };
        ((delegate* unmanaged[Stdcall]<IntPtr, IntPtr, uint, uint, uint, uint, IntPtr, uint, Caixa*, void>)Vt(ctx)[46])(ctx, copia, 0, 0, 0, 0, textura, 0, &caixa); // CopySubresourceRegion
        Mapeado m;
        if (((delegate* unmanaged[Stdcall]<IntPtr, IntPtr, uint, int, uint, Mapeado*, int>)Vt(ctx)[14])(ctx, copia, 0, 1 /* READ */, 0, &m) != 0) return false; // Map
        byte* linha = (byte*)m.Dados + 4 * m.Passo;
        for (int bit = 0; bit < 20; bit++)
        {
            int px = bit * 8 + 4;
            bool claro = d.Formato == 10
                ? (linha[px * 8] | (linha[px * 8 + 1] << 8)) > 0x3000
                : linha[px * 4] + linha[px * 4 + 1] + linha[px * 4 + 2] > 3 * 128;
            if (claro) n |= 1u << bit;
        }
        ((delegate* unmanaged[Stdcall]<IntPtr, IntPtr, uint, void>)Vt(ctx)[15])(ctx, copia, 0); // Unmap
        return true;
    }

    var vistos = new List<(uint n, long t)>(4096);
    int coletasAntes = GC.CollectionCount(0);
    double cpuAntes = CpuMs();
    var relogio = Stopwatch.StartNew();
    while (relogio.Elapsed.TotalSeconds < segundos)
    {
        InfoDoQuadro info;
        IntPtr recurso;
        int hr = ((delegate* unmanaged[Stdcall]<IntPtr, uint, InfoDoQuadro*, IntPtr*, int>)Vt(dup)[8])(dup, 100, &info, &recurso); // AcquireNextFrame
        if (hr == unchecked((int)0x887A0027)) continue; // DXGI_ERROR_WAIT_TIMEOUT
        if (hr != 0) { Console.WriteLine($"a duplicação parou: 0x{hr:X8}"); return 1; }
        Nativo.QueryPerformanceCounter(out long chegou);
        if (info.UltimoPresent != 0)
        {
            IntPtr t;
            if (((delegate* unmanaged[Stdcall]<IntPtr, Guid*, IntPtr*, int>)Vt(recurso)[0])(recurso, &iidTextura, &t) == 0)
            {
                if (LerNumero(t, out uint n)) vistos.Add((n, chegou));
                Liberar(t);
            }
        }
        Liberar(recurso);
        ((delegate* unmanaged[Stdcall]<IntPtr, int>)Vt(dup)[14])(dup); // ReleaseFrame
    }
    double cpu = CpuMs() - cpuAntes;
    int coletas = GC.CollectionCount(0) - coletasAntes;

    var presentes = new Dictionary<uint, long>();
    // O jogo ainda escreve no arquivo: abrir compartilhado (o padrão do C# recusa, o de Rust e C++ não).
    if (arquivo != null && File.Exists(arquivo))
        using (var leitor = new StreamReader(new FileStream(arquivo, FileMode.Open, FileAccess.Read, FileShare.ReadWrite)))
        for (string l; (l = leitor.ReadLine()) != null;)
        {
            var p = l.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            if (p.Length == 2 && uint.TryParse(p[0], out uint n) && long.TryParse(p[1], out long q)) presentes[n] = q;
        }
    double Ms(long q) => q * 1000.0 / freq;
    var novos = new List<(uint n, long t)>();
    int repetidos = 0, lixo = 0;
    foreach (var v in vistos)
    {
        if (!presentes.ContainsKey(v.n)) { lixo++; continue; }
        if (novos.Count > 0 && novos[^1].n >= v.n) repetidos++; else novos.Add(v);
    }
    uint primeiro = novos.Count > 0 ? novos[0].n : 0, ultimo = novos.Count > 0 ? novos[^1].n : 0;
    int doJogo = presentes.Keys.Count(k => k >= primeiro && k <= ultimo);
    int perdidos = Math.Max(0, doJogo - novos.Count);
    double duracao = novos.Count > 1 ? Math.Max(Ms(novos[^1].t - novos[0].t) / 1000.0, 1e-3) : 1e-3;
    var intervalos = new List<double>(); for (int i = 1; i < novos.Count; i++) intervalos.Add(Ms(novos[i].t - novos[i - 1].t));
    var atrasos = novos.Select(v => Ms(v.t - presentes[v.n])).ToList();
    static double Pct(List<double> v, double p) { if (v.Count == 0) return double.NaN; v.Sort(); return v[(int)Math.Round((v.Count - 1) * p)]; }
    var inv = CultureInfo.InvariantCulture;
    Console.WriteLine(string.Format(inv,
        "dxgi-c# | jogo {0:F1} fps | capturou {1:F1}/s novos ({2} quadros) | perdidos {3} ({4:F1}%) | repetidos {5} | lixo {6} | intervalo p50 {7:F2} ms p99 {8:F2} ms max {9:F2} ms | atraso p50 {10:F2} ms p95 {11:F2} ms | CPU {12:F3} ms por quadro | coletas do GC {13}",
        doJogo / duracao, novos.Count / duracao, novos.Count, perdidos, 100.0 * perdidos / Math.Max(doJogo, 1), repetidos, lixo,
        Pct(intervalos, 0.5), Pct(intervalos, 0.99), Pct(intervalos, 1.0), Pct(atrasos, 0.5), Pct(atrasos, 0.95),
        cpu / Math.Max(vistos.Count, 1), coletas));
    Nativo.timeEndPeriod(1);
    return 0;
}

static unsafe void** Vt(IntPtr objeto) => *(void***)objeto;
static unsafe void Liberar(IntPtr objeto) { if (objeto != IntPtr.Zero) ((delegate* unmanaged[Stdcall]<IntPtr, uint>)Vt(objeto)[2])(objeto); }
static void Checar(int hr, string onde) { if (hr != 0) throw new Exception($"{onde} falhou: 0x{hr:X8}"); }
static double CpuMs() { Nativo.GetProcessTimes(Nativo.GetCurrentProcess(), out _, out _, out long k, out long u); return (k + u) / 10000.0; }

[StructLayout(LayoutKind.Sequential)] struct Ponto { public int X, Y; }
[StructLayout(LayoutKind.Sequential)] struct Ret { public int Esquerda, Topo, Direita, Baixo; }
[StructLayout(LayoutKind.Sequential)] unsafe struct DescDaSaida { public fixed char Nome[32]; public Ret Area; public int NaAreaDeTrabalho; public int Rotacao; public IntPtr Monitor; }
[StructLayout(LayoutKind.Sequential)] struct InfoDoQuadro { public long UltimoPresent, UltimoMouse; public uint Acumulados; public int Juntados, Protegido, MouseX, MouseY, MouseVisivel; public uint Metadados, Ponteiro; }
[StructLayout(LayoutKind.Sequential)] struct DescDaTextura { public uint Largura, Altura, Mips, Camadas, Formato, Amostras, Qualidade, Uso, Ligacao, AcessoCpu, Outros; }
[StructLayout(LayoutKind.Sequential)] struct Caixa { public uint Esquerda, Topo, Frente, Direita, Baixo, Tras; }
[StructLayout(LayoutKind.Sequential)] struct Mapeado { public IntPtr Dados; public uint Passo, Profundidade; }

static unsafe class Nativo
{
    [DllImport("user32.dll")] public static extern IntPtr SetProcessDpiAwarenessContext(IntPtr contexto);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindowW(string classe, string titulo);
    [DllImport("user32.dll")] public static extern int ClientToScreen(IntPtr hwnd, ref Ponto p);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr hwnd, uint flags);
    [DllImport("kernel32.dll")] public static extern int QueryPerformanceCounter(out long valor);
    [DllImport("kernel32.dll")] public static extern int QueryPerformanceFrequency(out long valor);
    [DllImport("kernel32.dll")] public static extern IntPtr GetCurrentProcess();
    [DllImport("kernel32.dll")] public static extern int GetProcessTimes(IntPtr p, out long criado, out long saiu, out long nucleo, out long usuario);
    [DllImport("winmm.dll")] public static extern uint timeBeginPeriod(uint ms);
    [DllImport("winmm.dll")] public static extern uint timeEndPeriod(uint ms);
    [DllImport("dxgi.dll")] public static extern int CreateDXGIFactory1(ref Guid iid, out IntPtr fabrica);
    [DllImport("d3d11.dll")] public static extern int D3D11CreateDevice(IntPtr adaptador, int tipo, IntPtr software, uint flags, int* niveis, uint quantos, uint sdk, out IntPtr device, IntPtr nivel, out IntPtr ctx);
}
