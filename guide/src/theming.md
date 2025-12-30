# Theming and UI Assets

The Tasks UI is server-rendered HTML with a small amount of CSS and optional
JavaScript. Theming is handled through a class toggle in the layout.

## Key files

- `templates/layout.html` - base layout shared by pages.
- `static/app.css` - styles.
- `templates/tasks_list.html`, `templates/task_detail.html`, `templates/task_new.html`.

## Light and dark themes

The layout template sets a theme class on the root element. CSS rules then
apply the correct colors.

The UI does not rely on a SPA framework. This is intentional:
- Pages work without JavaScript.
- The system is easier to reason about.

### Code example: theme handling script

From `templates/layout.html`:

```html
<script>
  // Theme handling: light, dark, or system preference.
  (function() {
    const key = "theme";
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)");
    const baseHtmlLight = "min-h-full bg-slate-50 text-slate-900";
    const baseHtmlDark = "min-h-full bg-slate-950 text-slate-100";
    const baseBodyLight = "min-h-full bg-slate-50 text-slate-900 transition-colors duration-200";
    const baseBodyDark = "min-h-full bg-slate-950 text-slate-100 transition-colors duration-200";

    function readPref() {
      try { return localStorage.getItem(key) || "system"; }
      catch (_) { return "system"; }
    }

    function writePref(mode) {
      try { localStorage.setItem(key, mode); } catch (_) { /* ignore */ }
    }

    function applyTheme(mode) {
      const useDark = mode === "dark" || (mode === "system" && prefersDark.matches);

      // Reset base classes depending on target theme to avoid mixed states.
      document.documentElement.className = useDark ? baseHtmlDark : baseHtmlLight;
      if (document.body) {
        document.body.className = useDark ? baseBodyDark : baseBodyLight;
      }

      if (useDark) {
        document.documentElement.classList.add("dark");
        if (document.body) document.body.classList.add("dark");
        document.documentElement.style.colorScheme = "dark";
      } else {
        document.documentElement.classList.remove("dark");
        if (document.body) document.body.classList.remove("dark");
        document.documentElement.style.colorScheme = "light";
      }

      document.documentElement.dataset.theme = mode;
    }

    const initial = readPref();
    applyTheme(initial);

    prefersDark.addEventListener("change", () => {
      if (readPref() === "system") {
        applyTheme("system");
      }
    });

    window.__setTheme = (mode) => {
      writePref(mode);
      applyTheme(mode);
    };

    window.__getTheme = () => readPref();
  })();
</script>
```

## Assets

Static assets are served from `/static` by the interface layer.
This keeps the UI assets bundled with the service.

### Code example: CSS file location

From `static/app.css`:

```css
body { font-family: system-ui, sans-serif; margin: 2rem; }
nav a { margin-right: 1rem; }
input, button { font-size: 1rem; }
label { display: inline-block; margin-right: 1rem; }
```

## Exercise

- Add a new CSS class for a "warning" badge and use it in a template.
- Add a cookie to remember the user's theme choice.

Next: Observability and Errors.
