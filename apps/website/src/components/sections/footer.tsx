export function Footer() {
  return (
    <footer className="border-t border-white/[0.03] py-10 px-6">
      <div className="max-w-5xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-4">
        <span className="text-sm text-text-primary font-medium">UniKey</span>
        <nav className="flex items-center gap-6">
          {["GitHub", "Twitter", "Privacy", "Terms"].map((link) => (
            <a
              key={link}
              href="#"
              className="text-xs text-text-quaternary hover:text-text-secondary transition-colors"
            >
              {link}
            </a>
          ))}
        </nav>
        <span className="text-xs text-text-quaternary">&copy; {new Date().getFullYear()}</span>
      </div>
    </footer>
  );
}
