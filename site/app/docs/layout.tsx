// T-SITE-001.21: Docs layout — sidebar + content two-column structure
import { DocsSidebar } from './components/DocsSidebar';

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="docs-layout">
      <DocsSidebar />
      <main id="docs-content" className="docs-content markdown-content">
        {children}
      </main>
    </div>
  );
}
