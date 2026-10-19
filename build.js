#!/usr/bin/env node
// build.js — generates index.html with all course content embedded
// Run: node build.js

const fs = require("fs");
const path = require("path");

const COURSE_DIR = path.join(__dirname, "course");
const OUT_FILE = path.join(__dirname, "index.html");

// ── Course structure ──────────────────────────────────────────────────────────

const PARTS = [
  {
    id: "part-0",
    title: "PART 0 — Before You Write a Single Line",
    dir: "PART-0-mental-model",
    lessons: [
      { file: "0.1-how-principal-engineers-start.md", title: "0.1 — How a Principal Engineer Starts a Project" },
      { file: "0.2-reading-requirements.md",           title: "0.2 — Reading the Business Requirements Like an Engineer" },
      { file: "0.3-identifying-what-you-need.md",      title: "0.3 — Identifying What You Need Before You Need It" },
      { file: "0.4-naming-things.md",                  title: "0.4 — Naming Things (The Hardest Problem in Engineering)" },
    ],
  },
  {
    id: "part-1",
    title: "PART 1 — Designing the System",
    dir: "PART-1-design",
    lessons: [
      { file: "1.1-entity-design.md",    title: "1.1 — Entity Relationship Design" },
      { file: "1.2-database-schema.md",  title: "1.2 — The Database Schema" },
      { file: "1.3-api-contract.md",     title: "1.3 — API Contract Design" },
      { file: "1.4-env-example.md",      title: "1.4 — The .env.example File" },
    ],
  },
  {
    id: "part-2",
    title: "PART 2 — Authentication",
    dir: "PART-2-auth",
    lessons: [
      { file: "2.1-auth-vs-authz.md",      title: "2.1 — Auth vs AuthZ" },
      { file: "2.2-jwt-tokens.md",          title: "2.2 — JWT Tokens" },
      { file: "2.3-password-hashing.md",    title: "2.3 — Password Hashing" },
      { file: "2.4-auth-middleware.md",     title: "2.4 — Auth Middleware" },
    ],
  },
  {
    id: "part-3",
    title: "PART 3 — Authorization",
    dir: "PART-3-authorization",
    lessons: [
      { file: "3.1-rbac.md",               title: "3.1 — RBAC: Role-Based Access Control" },
      { file: "3.2-abac.md",               title: "3.2 — ABAC: Attribute-Based Access Control" },
      { file: "3.3-when-to-use-which.md",  title: "3.3 — When to Use RBAC, ABAC, or Both" },
      { file: "3.4-seeding.md",            title: "3.4 — Seeding Roles and Permissions" },
    ],
  },
  {
    id: "part-4",
    title: "PART 4 — Products and Pricing",
    dir: "PART-4-products-pricing",
    lessons: [
      { file: "4.1-product-table-design.md",  title: "4.1 — The Product Table Design" },
      { file: "4.2-subscription-plans.md",    title: "4.2 — Subscription Plans" },
      { file: "4.3-grandfathering.md",        title: "4.3 — Grandfathering Logic" },
      { file: "4.4-coupons.md",               title: "4.4 — Coupons and Promo Codes" },
    ],
  },
  {
    id: "part-5",
    title: "PART 5 — Stripe Integration",
    dir: "PART-5-stripe",
    lessons: [
      { file: "5.1-how-stripe-works.md",      title: "5.1 — How Stripe Works" },
      { file: "5.2-setup.md",                 title: "5.2 — Setting Up Stripe" },
      { file: "5.3-one-time-purchases.md",    title: "5.3 — One-Time Purchases" },
      { file: "5.4-subscriptions.md",         title: "5.4 — Subscriptions" },
      { file: "5.5-webhooks.md",              title: "5.5 — Webhooks" },
      { file: "5.6-free-trials.md",           title: "5.6 — Free Trials" },
    ],
  },
  {
    id: "part-6",
    title: "PART 6 — Testing Strategy",
    dir: "PART-6-testing",
    lessons: [
      { file: "6.1-what-to-test.md",              title: "6.1 — What to Test and What Not to Test" },
      { file: "6.2-testing-auth-permissions.md",  title: "6.2 — Testing Auth and Permissions" },
      { file: "6.3-testing-stripe.md",            title: "6.3 — Testing the Stripe Flow" },
      { file: "6.4-seeding-for-tests.md",         title: "6.4 — Seeding for Tests" },
    ],
  },
  {
    id: "part-7",
    title: "PART 7 — Multi-Tenancy",
    dir: "PART-7-multi-tenancy",
    lessons: [
      { file: "7.1-what-is-multi-tenancy.md",   title: "7.1 — What Multi-Tenancy Is" },
      { file: "7.2-organizations-table.md",      title: "7.2 — Adding the Organizations Table" },
      { file: "7.3-tenant-isolation.md",         title: "7.3 — Tenant Isolation in Every Query" },
      { file: "7.4-per-tenant-roles.md",         title: "7.4 — Per-Tenant Roles and Permissions" },
    ],
  },
  {
    id: "part-8",
    title: "PART 8 — Production Readiness",
    dir: "PART-8-production-readiness",
    lessons: [
      { file: "8.1-audit-logging.md",        title: "8.1 — Audit Logging" },
      { file: "8.2-error-handling.md",        title: "8.2 — Error Handling Strategy" },
      { file: "8.3-rate-limiting.md",         title: "8.3 — Rate Limiting" },
      { file: "8.4-soft-deletes.md",          title: "8.4 — Soft Deletes" },
      { file: "8.5-database-migrations.md",   title: "8.5 — Database Migrations" },
      { file: "8.6-final-env-checklist.md",   title: "8.6 — The Final .env.example" },
    ],
  },
  {
    id: "part-9",
    title: "PART 9 — The Rust Project",
    dir: "PART-9-rust",
    lessons: [
      { file: "9.1-why-rust.md",              title: "9.1 — Why Rust for Production Backend" },
      { file: "9.2-project-setup.md",         title: "9.2 — Project Setup: Axum + Tokio + SQLx" },
      { file: "9.3-database-layer.md",        title: "9.3 — The Database Layer in Rust" },
      { file: "9.4-auth-in-rust.md",          title: "9.4 — Authentication in Rust" },
      { file: "9.5-rbac-abac-in-rust.md",     title: "9.5 — RBAC + ABAC in Rust" },
      { file: "9.6-stripe-in-rust.md",        title: "9.6 — Stripe Integration in Rust" },
      { file: "9.7-full-route-handlers.md",   title: "9.7 — Full Route Handlers in Axum" },
      { file: "9.8-testing-in-rust.md",       title: "9.8 — Testing in Rust" },
      { file: "9.9-final-checklist.md",       title: "9.9 — Final Checklist: Deploy-Ready" },
    ],
  },
];

// ── Read all markdown files ───────────────────────────────────────────────────

const lessons = [];

for (const part of PARTS) {
  for (const lesson of part.lessons) {
    const filePath = path.join(COURSE_DIR, part.dir, lesson.file);
    let content = "";
    try {
      content = fs.readFileSync(filePath, "utf8");
    } catch {
      console.warn(`  MISSING: ${filePath}`);
      content = `# ${lesson.title}\n\n_This lesson has not been written yet._`;
    }
    lessons.push({
      id: lesson.file.replace(".md", ""),
      title: lesson.title,
      partTitle: part.title,
      partId: part.id,
      content,
    });
  }
}

console.log(`Loaded ${lessons.length} lessons.`);

// ── Escape content for embedding in JS string ─────────────────────────────────

function escapeForJs(str) {
  return str
    .replace(/\\/g, "\\\\")
    .replace(/`/g, "\\`")
    .replace(/\$/g, "\\$");
}

// ── Build the lessons data array ──────────────────────────────────────────────

const lessonsJs = lessons
  .map(
    (l) =>
      `{ id: ${JSON.stringify(l.id)}, title: ${JSON.stringify(l.title)}, partTitle: ${JSON.stringify(l.partTitle)}, partId: ${JSON.stringify(l.partId)}, content: \`${escapeForJs(l.content)}\` }`
  )
  .join(",\n    ");

// ── Build sidebar entries ─────────────────────────────────────────────────────

const sidebarHtml = PARTS.map((part) => {
  const items = part.lessons
    .map((lesson) => {
      const id = lesson.file.replace(".md", "");
      return `<li class="lesson-item" data-id="${id}">${lesson.title.replace(/^[\d.]+\s*—\s*/, "")}</li>`;
    })
    .join("\n          ");
  return `
        <div class="part-group" data-part="${part.id}">
          <div class="part-header">${part.title}</div>
          <ul class="lesson-list">
          ${items}
          </ul>
        </div>`;
}).join("\n");

// ── Generate full HTML ────────────────────────────────────────────────────────

const html = `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Backend Engineer Course</title>
<script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/highlight.js@11.9.0/lib/highlight.min.js"></script>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/highlight.js@11.9.0/styles/github-dark.min.css">
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

  :root {
    --bg: #0f1117;
    --sidebar-bg: #161b22;
    --sidebar-border: #21262d;
    --content-bg: #0f1117;
    --text: #e6edf3;
    --text-muted: #7d8590;
    --accent: #58a6ff;
    --accent-hover: #79c0ff;
    --part-header: #3d444d;
    --lesson-hover: #1c2128;
    --lesson-active: #1f3a5f;
    --lesson-active-text: #58a6ff;
    --border: #21262d;
    --code-bg: #161b22;
    --btn-bg: #21262d;
    --btn-hover: #30363d;
    --btn-primary: #1f6feb;
    --btn-primary-hover: #388bfd;
    --progress-bg: #21262d;
    --progress-fill: #58a6ff;
    --sidebar-width: 300px;
  }

  html, body { height: 100%; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: var(--bg); color: var(--text); }

  /* ── Layout ── */
  .app { display: flex; height: 100vh; overflow: hidden; }

  /* ── Sidebar ── */
  .sidebar {
    width: var(--sidebar-width);
    min-width: var(--sidebar-width);
    background: var(--sidebar-bg);
    border-right: 1px solid var(--sidebar-border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    padding: 20px 16px 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .sidebar-header h1 { font-size: 13px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.08em; }

  .progress-bar {
    margin-top: 10px;
    height: 4px;
    background: var(--progress-bg);
    border-radius: 2px;
    overflow: hidden;
  }
  .progress-fill { height: 100%; background: var(--progress-fill); border-radius: 2px; transition: width 0.3s ease; }
  .progress-label { margin-top: 5px; font-size: 11px; color: var(--text-muted); }

  .sidebar-nav { flex: 1; overflow-y: auto; padding: 8px 0; }
  .sidebar-nav::-webkit-scrollbar { width: 4px; }
  .sidebar-nav::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }

  .part-group { margin-bottom: 4px; }

  .part-header {
    padding: 8px 16px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    user-select: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .part-header::after { content: "▾"; font-size: 10px; transition: transform 0.2s; }
  .part-group.collapsed .part-header::after { transform: rotate(-90deg); }
  .part-group.collapsed .lesson-list { display: none; }

  .lesson-list { list-style: none; }

  .lesson-item {
    padding: 7px 16px 7px 24px;
    font-size: 13px;
    color: var(--text-muted);
    cursor: pointer;
    line-height: 1.4;
    transition: background 0.1s, color 0.1s;
    border-left: 2px solid transparent;
  }
  .lesson-item:hover { background: var(--lesson-hover); color: var(--text); }
  .lesson-item.active { background: var(--lesson-active); color: var(--lesson-active-text); border-left-color: var(--accent); font-weight: 500; }
  .lesson-item.done { color: var(--text); }
  .lesson-item.done::before { content: "✓ "; color: #3fb950; font-size: 11px; }

  /* ── Main ── */
  .main { flex: 1; display: flex; flex-direction: column; overflow: hidden; }

  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 32px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 12px;
  }

  .topbar-title { font-size: 14px; color: var(--text-muted); flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .topbar-title strong { color: var(--text); }

  .nav-buttons { display: flex; gap: 8px; flex-shrink: 0; }

  .btn {
    padding: 6px 14px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--btn-bg);
    color: var(--text);
    font-size: 13px;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }
  .btn:hover:not(:disabled) { background: var(--btn-hover); border-color: var(--accent); }
  .btn:disabled { opacity: 0.35; cursor: default; }
  .btn.primary { background: var(--btn-primary); border-color: var(--btn-primary); }
  .btn.primary:hover:not(:disabled) { background: var(--btn-primary-hover); border-color: var(--btn-primary-hover); }

  .content-area { flex: 1; overflow-y: auto; padding: 48px 64px; }
  .content-area::-webkit-scrollbar { width: 6px; }
  .content-area::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }

  .lesson-content { max-width: 780px; margin: 0 auto; }

  /* ── Markdown rendering ── */
  .lesson-content h1 { font-size: 28px; font-weight: 700; margin-bottom: 24px; color: var(--text); line-height: 1.3; }
  .lesson-content h2 { font-size: 20px; font-weight: 600; margin: 36px 0 14px; color: var(--text); border-bottom: 1px solid var(--border); padding-bottom: 8px; }
  .lesson-content h3 { font-size: 16px; font-weight: 600; margin: 28px 0 10px; color: var(--text); }
  .lesson-content h4 { font-size: 14px; font-weight: 600; margin: 20px 0 8px; color: var(--text-muted); }
  .lesson-content p { font-size: 15px; line-height: 1.75; color: var(--text); margin-bottom: 16px; }
  .lesson-content ul, .lesson-content ol { padding-left: 24px; margin-bottom: 16px; }
  .lesson-content li { font-size: 15px; line-height: 1.75; color: var(--text); margin-bottom: 4px; }
  .lesson-content li > ul, .lesson-content li > ol { margin-top: 4px; margin-bottom: 4px; }
  .lesson-content strong { color: var(--text); font-weight: 600; }
  .lesson-content em { color: var(--accent); font-style: normal; font-weight: 500; }
  .lesson-content a { color: var(--accent); text-decoration: none; }
  .lesson-content a:hover { text-decoration: underline; }
  .lesson-content blockquote { border-left: 3px solid var(--accent); padding: 4px 16px; margin: 20px 0; color: var(--text-muted); background: var(--sidebar-bg); border-radius: 0 6px 6px 0; }
  .lesson-content blockquote p { color: var(--text-muted); margin-bottom: 0; }
  .lesson-content hr { border: none; border-top: 1px solid var(--border); margin: 32px 0; }
  .lesson-content table { width: 100%; border-collapse: collapse; margin: 20px 0; font-size: 14px; }
  .lesson-content th { background: var(--sidebar-bg); padding: 10px 14px; text-align: left; font-weight: 600; border: 1px solid var(--border); color: var(--text); }
  .lesson-content td { padding: 9px 14px; border: 1px solid var(--border); color: var(--text); }
  .lesson-content tr:nth-child(even) td { background: rgba(255,255,255,0.02); }

  .lesson-content code {
    font-family: "JetBrains Mono", "Fira Code", "Cascadia Code", Menlo, monospace;
    font-size: 13px;
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 6px;
    color: #f0883e;
  }

  .lesson-content pre {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 20px;
    overflow-x: auto;
    margin: 20px 0;
  }
  .lesson-content pre::-webkit-scrollbar { height: 4px; }
  .lesson-content pre::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }
  .lesson-content pre code {
    background: none;
    border: none;
    padding: 0;
    color: var(--text);
    font-size: 13px;
    line-height: 1.6;
  }

  /* ── Bottom nav ── */
  .bottom-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 64px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }
  .lesson-counter { font-size: 13px; color: var(--text-muted); }

  /* ── Welcome screen ── */
  .welcome { text-align: center; padding: 80px 32px; }
  .welcome h2 { font-size: 28px; margin-bottom: 12px; }
  .welcome p { font-size: 16px; color: var(--text-muted); margin-bottom: 28px; }
</style>
</head>
<body>
<div class="app">

  <!-- Sidebar -->
  <aside class="sidebar">
    <div class="sidebar-header">
      <h1>Backend Engineer Course</h1>
      <div class="progress-bar"><div class="progress-fill" id="progressFill" style="width:0%"></div></div>
      <div class="progress-label" id="progressLabel">0 of ${lessons.length} lessons complete</div>
    </div>
    <nav class="sidebar-nav" id="sidebarNav">
${sidebarHtml}
    </nav>
  </aside>

  <!-- Main -->
  <div class="main">
    <div class="topbar">
      <div class="topbar-title" id="topbarTitle">Select a lesson to begin</div>
      <div class="nav-buttons">
        <button class="btn" id="btnPrev" onclick="navigate(-1)" disabled>← Previous</button>
        <button class="btn primary" id="btnNext" onclick="navigate(1)" disabled>Next →</button>
      </div>
    </div>

    <div class="content-area" id="contentArea">
      <div class="lesson-content">
        <div class="welcome">
          <h2>Welcome to the Course</h2>
          <p>Select a lesson from the sidebar to begin, or pick up where you left off.</p>
          <button class="btn primary" onclick="resumeOrStart()">Start from the beginning</button>
        </div>
      </div>
    </div>

    <div class="bottom-nav">
      <button class="btn" id="btnPrev2" onclick="navigate(-1)" disabled>← Previous</button>
      <span class="lesson-counter" id="lessonCounter"></span>
      <button class="btn primary" id="btnNext2" onclick="navigate(1)" disabled>Next →</button>
    </div>
  </div>
</div>

<script>
// ── Data ──────────────────────────────────────────────────────────────────────
const LESSONS = [
    ${lessonsJs}
];

// ── State ─────────────────────────────────────────────────────────────────────
let currentIndex = -1;
let done = new Set(JSON.parse(localStorage.getItem("done") || "[]"));

// ── Marked config ─────────────────────────────────────────────────────────────
marked.setOptions({
  highlight: function(code, lang) {
    if (lang && hljs.getLanguage(lang)) {
      return hljs.highlight(code, { language: lang }).value;
    }
    return hljs.highlightAuto(code).value;
  },
  breaks: false,
  gfm: true,
});

// ── Render ────────────────────────────────────────────────────────────────────
function renderLesson(index) {
  if (index < 0 || index >= LESSONS.length) return;

  const lesson = LESSONS[index];
  currentIndex = index;

  // Content
  document.getElementById("contentArea").innerHTML =
    '<div class="lesson-content">' + marked.parse(lesson.content) + '</div>';
  document.getElementById("contentArea").scrollTop = 0;

  // Topbar
  document.getElementById("topbarTitle").innerHTML =
    '<strong>' + escHtml(lesson.partTitle) + '</strong> &nbsp;·&nbsp; ' + escHtml(lesson.title);

  // Counter
  document.getElementById("lessonCounter").textContent =
    'Lesson ' + (index + 1) + ' of ' + LESSONS.length;

  // Buttons
  const hasPrev = index > 0;
  const hasNext = index < LESSONS.length - 1;
  document.getElementById("btnPrev").disabled  = !hasPrev;
  document.getElementById("btnNext").disabled  = !hasNext;
  document.getElementById("btnPrev2").disabled = !hasPrev;
  document.getElementById("btnNext2").disabled = !hasNext;

  // Sidebar active state
  document.querySelectorAll(".lesson-item").forEach(el => el.classList.remove("active"));
  const activeEl = document.querySelector('[data-id="' + lesson.id + '"]');
  if (activeEl) {
    activeEl.classList.add("active");
    // Expand the part group if collapsed
    const group = activeEl.closest(".part-group");
    if (group) group.classList.remove("collapsed");
    // Scroll sidebar to active item
    activeEl.scrollIntoView({ block: "nearest" });
  }

  // Mark as done
  done.add(lesson.id);
  saveDone();
  updateProgress();

  // Save position
  localStorage.setItem("position", index);

  // Syntax highlight
  document.querySelectorAll("pre code").forEach(el => {
    if (!el.classList.contains("hljs")) hljs.highlightElement(el);
  });
}

function navigate(dir) {
  const next = currentIndex + dir;
  if (next >= 0 && next < LESSONS.length) renderLesson(next);
}

function resumeOrStart() {
  const saved = parseInt(localStorage.getItem("position") || "0", 10);
  renderLesson(isNaN(saved) ? 0 : saved);
}

// ── Sidebar ───────────────────────────────────────────────────────────────────
function buildSidebar() {
  LESSONS.forEach((lesson, index) => {
    const el = document.querySelector('[data-id="' + lesson.id + '"]');
    if (!el) return;
    if (done.has(lesson.id)) el.classList.add("done");
    el.addEventListener("click", () => renderLesson(index));
  });

  // Part group collapse/expand
  document.querySelectorAll(".part-header").forEach(header => {
    header.addEventListener("click", () => {
      header.closest(".part-group").classList.toggle("collapsed");
    });
  });
}

// ── Progress ──────────────────────────────────────────────────────────────────
function updateProgress() {
  const pct = Math.round((done.size / LESSONS.length) * 100);
  document.getElementById("progressFill").style.width = pct + "%";
  document.getElementById("progressLabel").textContent =
    done.size + " of " + LESSONS.length + " lessons complete";

  // Update done markers in sidebar
  LESSONS.forEach(lesson => {
    const el = document.querySelector('[data-id="' + lesson.id + '"]');
    if (!el) return;
    if (done.has(lesson.id)) el.classList.add("done");
  });
}

function saveDone() {
  localStorage.setItem("done", JSON.stringify([...done]));
}

// ── Keyboard navigation ───────────────────────────────────────────────────────
document.addEventListener("keydown", e => {
  if (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA") return;
  if (e.key === "ArrowRight" || e.key === "l") navigate(1);
  if (e.key === "ArrowLeft"  || e.key === "h") navigate(-1);
});

// ── Utilities ─────────────────────────────────────────────────────────────────
function escHtml(str) {
  return str.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;");
}

// ── Init ──────────────────────────────────────────────────────────────────────
buildSidebar();
updateProgress();

// Auto-resume if there's a saved position
const savedPos = parseInt(localStorage.getItem("position") || "-1", 10);
if (savedPos >= 0 && savedPos < LESSONS.length) {
  renderLesson(savedPos);
}
</script>
</body>
</html>`;

fs.writeFileSync(OUT_FILE, html, "utf8");

const sizeKb = Math.round(fs.statSync(OUT_FILE).size / 1024);
console.log(`Built: index.html (${sizeKb} KB)`);
console.log(`Open:  open "${OUT_FILE}"`);
