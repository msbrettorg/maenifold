import fs from 'fs';
import path from 'path';

interface Tool {
  name: string;
  description: string;
  slug: string;
}

function extractToolData(content: string, filename: string): Tool {
  // Extract H1 title
  const titleMatch = content.match(/^# (.+)$/m);
  const name = titleMatch ? titleMatch[1] : filename.replace('.md', '');

  // Extract first paragraph (everything after H1 until next section or empty line with no content)
  const paragraphMatch = content.match(/^# .+\n\n([\s\S]*?)(?:\n##|\n\n\n|$)/);
  const description = paragraphMatch
    ? paragraphMatch[1]
      .trim()
      .split('\n')
      .slice(0, 2)
      .join(' ')
      .replace(/\[\[.*?\]\]/g, '') // Remove [[WikiLink]] references
      .trim()
    : 'Tool documentation';

  // Convert filename to kebab-case slug (e.g., writememory -> write-memory)
  const slug = filename
    .replace('.md', '')
    .replace(/([a-z])([A-Z])/g, '$1-$2')
    .toLowerCase();

  return { name, description, slug };
}

export default function ToolsPage() {
  // Load all tool markdown files
  const toolsDir = path.join(process.cwd(), '../src/assets/usage/tools');
  const toolFiles = fs
    .readdirSync(toolsDir)
    .filter((file) => file.endsWith('.md'))
    .sort();

  const tools: Tool[] = toolFiles.map((filename) => {
    const filepath = path.join(toolsDir, filename);
    const content = fs.readFileSync(filepath, 'utf-8');
    return extractToolData(content, filename);
  });

  return (
    <div className="bg-white dark:bg-slate-900 text-slate-900 dark:text-white">
      {/* Page Header */}
      <div className="max-w-7xl mx-auto px-4 py-12 border-b border-slate-200 dark:border-slate-700">
        <h1 className="text-4xl md:text-5xl font-bold mb-4">Maenifold Tools</h1>
        <p className="text-lg text-slate-600 dark:text-slate-300">
          {tools.length} MCP tools for persistent knowledge management, graph operations, and AI orchestration.
        </p>
      </div>

      {/* Tools Grid */}
      <div className="max-w-7xl mx-auto px-4 py-12">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {tools.map((tool) => (
            <a
              key={tool.slug}
              href={`/maenifold/tools/${tool.slug}`}
              className="group border border-slate-200 dark:border-slate-700 rounded-lg p-6 hover:border-blue-400 dark:hover:border-blue-500 hover:shadow-lg dark:hover:shadow-blue-500/10 transition-all"
            >
              <h2 className="text-xl font-bold mb-3 text-slate-900 dark:text-white group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors">
                {tool.name}
              </h2>
              <p className="text-slate-600 dark:text-slate-300 line-clamp-2">
                {tool.description}
              </p>
              <div className="mt-4 text-blue-600 dark:text-blue-400 font-semibold text-sm group-hover:translate-x-1 transition-transform">
                View tool →
              </div>
            </a>
          ))}
        </div>
      </div>

      {/* Info Section */}
      <div className="max-w-7xl mx-auto px-4 py-12 border-t border-slate-200 dark:border-slate-700">
        <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-6">
          <h2 className="text-2xl font-bold mb-4">About These Tools</h2>
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            Maenifold provides a comprehensive toolkit for knowledge graph operations, semantic search, concept repair, and complex reasoning workflows.
          </p>
          <p className="text-slate-700 dark:text-slate-300">
            Each tool is available as an MCP (Model Context Protocol) server and can be integrated into your Claude projects, Continue IDE extensions, and Cline workflows.
          </p>
        </div>
      </div>
    </div>
  );
}
