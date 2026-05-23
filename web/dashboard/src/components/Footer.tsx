export default function Footer() {
  const buildLabel = __AETHER_DASHBOARD_BUILD__.replace('T', ' ').slice(0, 19);

  return (
    <footer className="mt-auto border-t border-slate-800/40 py-3 text-center">
      <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-slate-600">
        UI build {buildLabel}
      </p>
    </footer>
  );
}
