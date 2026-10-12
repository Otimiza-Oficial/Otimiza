// Laboratório E2: três jeitos de entregar quadros a um ritmo fixo, para medir o tranco com a bancada de captura
// (src-tauri/examples/bancada_captura.rs, que lê o número de cada quadro na tela). A janela tem a classe "JogoDeTeste"
// e o mesmo marcador (20 quadradinhos de 8 px no canto) e arquivo "número instante" do jogo de teste.
//
//   cronometro <fps>: como os modos 2×/3×/4× do Frame Boost hoje: composição DirectComposition, Present(0) por relógio.
//   monitor:          como o Auto hoje: espera a vez do monitor (objeto de latência) e Present(1), um quadro por atualização.
//   horario <fps>:    Composition Swapchain (Windows 11): cada quadro marcado para a atualização exata do monitor.
//
//   apresentador.exe <modo> <fps> <segundos> <arquivo do marcador>
//   (posição: OTIMIZA_TESTE_X / OTIMIZA_TESTE_Y; tamanho 640x360)
#define NOMINMAX
#include <windows.h>
#include <d3d11_1.h>
#include <dxgi1_3.h>
#include <dcomp.h>
#include <dwmapi.h>
#include <presentation.h>
#include <realtimeapiset.h>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>

#pragma comment(lib, "d3d11.lib")
#pragma comment(lib, "dxgi.lib")
#pragma comment(lib, "dcomp.lib")
#pragma comment(lib, "dwmapi.lib")
#pragma comment(lib, "winmm.lib")
#pragma comment(lib, "user32.lib")
#pragma comment(lib, "mincore.lib")

static const UINT LARGURA = 640, ALTURA = 360;
static LARGE_INTEGER freq;
static int64_t qpc() { LARGE_INTEGER v; QueryPerformanceCounter(&v); return v.QuadPart; }

static LRESULT CALLBACK proc(HWND h, UINT m, WPARAM w, LPARAM l) {
    if (m == WM_NCHITTEST) return HTTRANSPARENT;
    if (m == WM_MOUSEACTIVATE) return MA_NOACTIVATE;
    return DefWindowProcW(h, m, w, l);
}

static bool bombear() {
    MSG msg;
    while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) {
        if (msg.message == WM_QUIT) return false;
        DispatchMessageW(&msg);
    }
    return true;
}

// O número do quadro no canto (o mesmo marcador do jogo de teste) e um fundo que muda de cor.
static void desenhar(ID3D11DeviceContext1* ctx, ID3D11RenderTargetView* rtv, uint32_t n) {
    float fundo[4] = {0.1f + 0.3f * (n % 7) / 7.0f, 0.15f, 0.2f, 1.0f};
    ctx->ClearRenderTargetView(rtv, fundo);
    for (int bit = 0; bit < 20; ++bit) {
        float cor[4];
        float v = ((n >> bit) & 1) ? 1.0f : 0.0f;
        cor[0] = cor[1] = cor[2] = v; cor[3] = 1.0f;
        D3D11_RECT r{bit * 8, 0, bit * 8 + 8, 8};
        ctx->ClearView(rtv, cor, &r, 1);
    }
}

int main(int argc, char** argv) {
    if (argc < 5) { std::printf("uso: apresentador <cronometro|monitor|horario> <fps> <segundos> <marcador>\n"); return 1; }
    const char* modo = argv[1];
    const double fps = atof(argv[2]);
    const double segundos = atof(argv[3]);
    FILE* marcador = fopen(argv[4], "w");
    QueryPerformanceFrequency(&freq);
    timeBeginPeriod(1);
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    const char* ex = getenv("OTIMIZA_TESTE_X");
    const char* ey = getenv("OTIMIZA_TESTE_Y");
    const int x = ex ? atoi(ex) : 1280, y = ey ? atoi(ey) : 0;

    WNDCLASSW wc{};
    wc.lpfnWndProc = proc;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.lpszClassName = L"JogoDeTeste";
    RegisterClassW(&wc);
    HWND hwnd = CreateWindowExW(WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP, L"JogoDeTeste",
                                L"Apresentador do laboratório", WS_POPUP, x, y, LARGURA, ALTURA, nullptr, nullptr, wc.hInstance, nullptr);
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);

    const bool horario = strcmp(modo, "horario") == 0;
    UINT flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
    if (horario) flags |= D3D11_CREATE_DEVICE_SINGLETHREADED | D3D11_CREATE_DEVICE_PREVENT_INTERNAL_THREADING_OPTIMIZATIONS;
    ID3D11Device* dev = nullptr;
    ID3D11DeviceContext* ctx0 = nullptr;
    if (FAILED(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr, flags, nullptr, 0, D3D11_SDK_VERSION, &dev, nullptr, &ctx0))) {
        std::printf("D3D11 falhou\n"); return 1;
    }
    ID3D11DeviceContext1* ctx = nullptr;
    ctx0->QueryInterface(&ctx);
    IDXGIDevice* dxgi = nullptr;
    dev->QueryInterface(&dxgi);
    IDCompositionDevice* dcomp = nullptr;
    DCompositionCreateDevice(dxgi, __uuidof(IDCompositionDevice), reinterpret_cast<void**>(&dcomp));
    IDCompositionTarget* alvo = nullptr;
    dcomp->CreateTargetForHwnd(hwnd, TRUE, &alvo);
    IDCompositionVisual* visual = nullptr;
    dcomp->CreateVisual(&visual);

    // O ritmo do monitor (para o modo horário).
    DWM_TIMING_INFO ti{};
    ti.cbSize = sizeof(ti);
    DwmGetCompositionTimingInfo(nullptr, &ti);
    const double hz = ti.qpcRefreshPeriod ? static_cast<double>(freq.QuadPart) / ti.qpcRefreshPeriod : 60.0;

    const int64_t inicio = qpc();
    const int64_t fim = inicio + static_cast<int64_t>(segundos * freq.QuadPart);
    uint32_t n = 0;
    // Grava aos poucos: a bancada lê o arquivo com o apresentador ainda rodando.
    auto anotar = [&](uint32_t numero) {
        if (!marcador) return;
        fprintf(marcador, "%u %lld\n", numero, static_cast<long long>(qpc()));
        if (numero % 30 == 0) fflush(marcador);
    };

    if (!horario) {
        const bool pelo_monitor = strcmp(modo, "monitor") == 0;
        IDXGIAdapter* adaptador = nullptr;
        dxgi->GetAdapter(&adaptador);
        IDXGIFactory2* fabrica = nullptr;
        adaptador->GetParent(__uuidof(IDXGIFactory2), reinterpret_cast<void**>(&fabrica));
        DXGI_SWAP_CHAIN_DESC1 d{};
        d.Width = LARGURA; d.Height = ALTURA; d.Format = DXGI_FORMAT_B8G8R8A8_UNORM; d.SampleDesc.Count = 1;
        d.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT; d.BufferCount = 2; d.Scaling = DXGI_SCALING_STRETCH;
        d.SwapEffect = DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL; d.AlphaMode = DXGI_ALPHA_MODE_IGNORE;
        d.Flags = pelo_monitor ? DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT : 0;
        IDXGISwapChain1* swap = nullptr;
        if (FAILED(fabrica->CreateSwapChainForComposition(dev, &d, nullptr, &swap))) { std::printf("swap chain falhou\n"); return 1; }
        HANDLE vez = nullptr;
        if (pelo_monitor) {
            IDXGISwapChain2* s2 = nullptr;
            swap->QueryInterface(&s2);
            s2->SetMaximumFrameLatency(1);
            vez = s2->GetFrameLatencyWaitableObject();
        }
        visual->SetContent(swap);
        alvo->SetRoot(visual);
        dcomp->Commit();
        ID3D11Texture2D* fundo = nullptr;
        swap->GetBuffer(0, __uuidof(ID3D11Texture2D), reinterpret_cast<void**>(&fundo));
        ID3D11RenderTargetView* rtv = nullptr;
        dev->CreateRenderTargetView(fundo, nullptr, &rtv);
        const int64_t passo = static_cast<int64_t>(freq.QuadPart / fps);
        int64_t proximo = qpc();
        while (qpc() < fim && bombear()) {
            if (pelo_monitor) {
                WaitForSingleObjectEx(vez, 100, TRUE);
            } else {
                // Por relógio, como os modos fixos: dorme até perto da hora e fecha a espera girando.
                proximo += passo;
                for (int64_t falta; (falta = proximo - qpc()) > 0;) {
                    if (falta > freq.QuadPart / 500) Sleep(1);
                }
            }
            desenhar(ctx, rtv, ++n);
            swap->Present(pelo_monitor ? 1 : 0, 0);
            anotar(n);
        }
    } else {
        IPresentationFactory* fabrica = nullptr;
        if (FAILED(CreatePresentationFactory(dev, __uuidof(IPresentationFactory), reinterpret_cast<void**>(&fabrica))) ||
            !fabrica->IsPresentationSupported()) {
            std::printf("Composition Swapchain indisponivel\n"); return 1;
        }
        IPresentationManager* gerente = nullptr;
        fabrica->CreatePresentationManager(&gerente);
        HANDLE alca = nullptr;
        DCompositionCreateSurfaceHandle(COMPOSITIONOBJECT_ALL_ACCESS, nullptr, &alca);
        IPresentationSurface* superficie = nullptr;
        gerente->CreatePresentationSurface(alca, &superficie);
        // Sem o retângulo de origem a superfície não mostra nada; sem o modo de transparência, o fundo pode sumir.
        RECT origem_da_imagem{0, 0, static_cast<LONG>(LARGURA), static_cast<LONG>(ALTURA)};
        superficie->SetSourceRect(&origem_da_imagem);
        superficie->SetAlphaMode(DXGI_ALPHA_MODE_IGNORE);
        IUnknown* dsuperficie = nullptr;
        IDCompositionDevice* dcompv1 = dcomp;
        IDCompositionDesktopDevice* desk = nullptr;
        // CreateSurfaceFromHandle existe no IDCompositionDevice.
        dcompv1->CreateSurfaceFromHandle(alca, &dsuperficie);
        visual->SetContent(dsuperficie);
        alvo->SetRoot(visual);
        dcomp->Commit();
        (void)desk;

        const int QUANTOS = 4;
        std::vector<ID3D11Texture2D*> texturas(QUANTOS);
        std::vector<ID3D11RenderTargetView*> rtvs(QUANTOS);
        std::vector<IPresentationBuffer*> buffers(QUANTOS);
        std::vector<HANDLE> eventos;
        HANDLE perdido = nullptr;
        gerente->GetLostEvent(&perdido);
        eventos.push_back(perdido);
        for (int i = 0; i < QUANTOS; ++i) {
            D3D11_TEXTURE2D_DESC td{};
            td.Width = LARGURA; td.Height = ALTURA; td.MipLevels = 1; td.ArraySize = 1; td.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
            td.SampleDesc.Count = 1; td.Usage = D3D11_USAGE_DEFAULT; td.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;
            td.MiscFlags = D3D11_RESOURCE_MISC_SHARED | D3D11_RESOURCE_MISC_SHARED_NTHANDLE | D3D11_RESOURCE_MISC_SHARED_DISPLAYABLE;
            dev->CreateTexture2D(&td, nullptr, &texturas[i]);
            dev->CreateRenderTargetView(texturas[i], nullptr, &rtvs[i]);
            // No SDK 10.0.26100 a textura entra direto (AddBufferFromResource); a documentação ainda mostra o
            // AddBufferFromSharedHandle de uma versão anterior.
            if (FAILED(gerente->AddBufferFromResource(texturas[i], &buffers[i]))) { std::printf("buffer recusado\n"); return 1; }
            HANDLE livre = nullptr;
            buffers[i]->GetAvailableEvent(&livre);
            eventos.push_back(livre);
        }

        // A grade das atualizações do monitor, em unidades de 100 ns do relógio do sistema (QPC / freq * 10^7).
        auto em_100ns = [&](int64_t q) { return static_cast<uint64_t>(static_cast<double>(q) * 1e7 / freq.QuadPart); };
        DwmGetCompositionTimingInfo(nullptr, &ti);
        const int64_t periodo = ti.qpcRefreshPeriod;
        // Começa algumas atualizações à frente, para a fila ter folga.
        const int64_t base = static_cast<int64_t>(ti.qpcVBlank) + 4 * periodo;
        const double atualizacoes_por_quadro = hz / fps;
        while (qpc() < fim && bombear()) {
            DWORD r = WaitForMultipleObjects(static_cast<DWORD>(eventos.size()), eventos.data(), FALSE, 100);
            if (r == WAIT_TIMEOUT) continue;
            if (r == WAIT_OBJECT_0) { std::printf("gerente de apresentacao perdido\n"); break; }
            const UINT i = r - (WAIT_OBJECT_0 + 1);
            ++n;
            // O quadro n vai na primeira atualização a partir da sua hora ideal; mira meio período antes dela. A grade
            // vem do DWM a cada quadro (calculada uma vez só, um erro mínimo no período se acumula e vira tranco).
            (void)atualizacoes_por_quadro;
            DwmGetCompositionTimingInfo(nullptr, &ti);
            const int64_t ideal = base + static_cast<int64_t>((n - 1) * static_cast<double>(freq.QuadPart) / fps);
            const int64_t vb = static_cast<int64_t>(ti.qpcVBlank), per = static_cast<int64_t>(ti.qpcRefreshPeriod);
            const int64_t k = (ideal - vb + per - 1) / per;
            const int64_t mira = vb + k * per - per / 2;
            // Fila curta, como o Frame Boost precisa: só entrega quando faltam no máximo 2 atualizações para a hora.
            while (mira - qpc() > 2 * periodo) {
                if (mira - qpc() > 3 * periodo) Sleep(1);
            }
            desenhar(ctx, rtvs[i], n);
            superficie->SetBuffer(buffers[i]);
            SystemInterruptTime t{em_100ns(mira)};
            gerente->SetTargetTime(t);
            if (gerente->Present() == PRESENTATION_ERROR_LOST) { std::printf("perdido no present\n"); break; }
            anotar(n);
        }
    }
    if (marcador) fclose(marcador);
    std::printf("%s: %u quadros a %.1f fps pedidos (monitor de %.1f Hz)\n", modo, n, fps, hz);
    timeEndPeriod(1);
    return 0;
}
