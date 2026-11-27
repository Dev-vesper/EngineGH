# Engine

موتور بازی شخصی با Rust (wgpu + glam + hecs). همه‌چیز سورس همین ریپو است و قابل تغییر؛ بازی می‌تواند از لایه‌ی `engine_app` استفاده کند یا مستقیم به crateهای پایین‌تر وصل شود.

## ساختار

```
crates/   خود انجین (کتابخانه‌ها)
  engine_platform   پنجره، event loop، ورودی، حلقه‌ی گام ثابت (تنها crate وابسته به winit)
  engine_render     wgpu: سطح، MSAA، depth، پایپلاین، instancing
  engine_camera     دوربین fly (فقط glam)
  engine_time       FixedTimestep و SimControl (مکث / گام‌به‌گام / سرعت زمان)
  engine_scene      کامپوننت‌ها (Transform...) و سیستم‌های اتصال ECS به رندر
  engine_memory     ردیاب heap
  engine_editor     ادیتور ImGui داخل برنامه (پنل‌ها)
  engine_app        لایه‌ی چارچوب: پنجره + رندر + ادیتور + ECS پشت trait ‏`Game`
games/    بازی‌ها (هر پوشه یک برنامه)
  sandbox           بازی نمونه و الگوی بازی‌های جدید
apps/     ابزارها
  benchmark         بنچمارک انتها‌به‌انتها با خروجی JSON
benchmarks/results/ گزارش‌های بنچمارک
```

وابستگی‌ها یک‌طرفه‌اند: `games` و `apps` ← `engine_app` ← بقیه. `engine_render` از winit خبر ندارد.

## ساخت بازی جدید

```bash
cp -r games/sandbox games/mygame
# در games/mygame/Cargo.toml مقدار name را به mygame تغییر دهید (workspace خودش پوشه را می‌شناسد)
cargo run -p mygame --release
```

بازی فقط `Game` را پیاده می‌کند:

| متد | کاربرد |
|---|---|
| `setup(&mut Engine)` | یک‌بار قبل از باز شدن پنجره: ساخت entityها، دوربین، محیط |
| `fixed_update(&mut Engine, dt)` | گیم‌پلی و فیزیک با نرخ ثابت (پیش‌فرض ۶۰Hz)؛ Transformها خودکار interpolate می‌شوند |
| `update(&mut Engine, &Input, dt)` | هر فریم: دوربین، ورودی، هرچه شبیه‌سازی نیست |

`Engine` شامل `world` (ECS)، `camera`، `env`، `sim` و `quit` است و همه‌ی فیلدها public هستند.
کامپوننت یک struct ساده است و سیستم یک تابع روی `World` (نمونه: `spin_system` در sandbox).

## ادیتور داخل بازی

وقتی بازی اجرا می‌شود ادیتور هم داخل همان برنامه است:

- **F1**: نمایش/پنهان‌کردن ادیتور
- **کلیک چپ روی صحنه**: قفل ماوس؛ **Esc**: آزاد کردن ماوس (اگر آزاد بود، خروج)
- **WASD / Space / Ctrl / Shift**: حرکت دوربین
- منوی **View**: روشن/خاموش کردن پنل‌ها (Performance, Renderer, Memory, Simulation, Camera, Environment, Scene)

برای نسخه‌ی نهایی بدون ادیتور و بدون ردیاب حافظه:

```bash
cargo run -p mygame --release --no-default-features
```

## بنچمارک

```bash
cargo run -p benchmark --release                       # ماتریس پیش‌فرض
cargo run -p benchmark --release -- --help
```

گزارش JSON در `benchmarks/results/` ذخیره می‌شود. اگر چند نتیجه روی نرخ نمایشگر قفل شدند، آن اجراها display-bound هستند؛ `--instances` بزرگ‌تر بدهید.

## نیازمندی‌ها

Rust جدید (wgpu 30)، کامپایلر C++ (برای ImGui). روی دبیان: `build-essential`.
