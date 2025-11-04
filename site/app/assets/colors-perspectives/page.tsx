import { getAllColors, getAllPerspectives } from '@/lib/assets';
import { GlassCard } from '@/app/components/GlassCard';

export default function ColorsPerspectivesPage() {
  const colors = getAllColors();
  const perspectives = getAllPerspectives();

  return (
    <div className="min-h-screen bg-gradient-to-br from-white via-blue-50 to-white dark:from-slate-900 dark:via-slate-800 dark:to-slate-900 px-6 py-16">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="text-center mb-12">
          <h1 className="text-4xl md:text-5xl font-bold mb-4 text-slate-900 dark:text-white">
            🎨 Colors & Perspectives
          </h1>
          <p className="text-lg text-slate-700 dark:text-slate-300 max-w-3xl mx-auto">
            19 thinking modes and linguistic lenses. De Bono's Six Thinking Hats provide structured
            cognitive modes, while 12 linguistic perspectives offer diverse cultural approaches to problem-solving.
          </p>
        </div>

        {/* Colors (Thinking Hats) */}
        <div className="mb-16">
          <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">
            Thinking Colors ({colors.length})
          </h2>
          <p className="text-sm text-slate-600 dark:text-slate-400 mb-6">
            De Bono's Six Thinking Hats plus Gray - cognitive modes for facts, emotions, caution,
            optimism, creativity, skeptical inquiry, and process control.
          </p>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {colors.map((color) => (
              <GlassCard key={color.id} className="p-6 hover:scale-105 transition-transform">
                <div className="flex items-start gap-3 mb-3">
                  <div className="text-3xl">{color.emoji}</div>
                  <div className="flex-1">
                    <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-1">
                      {color.name}
                    </h3>
                    <p className="text-sm text-slate-600 dark:text-slate-400">
                      {color.description}
                    </p>
                  </div>
                </div>
              </GlassCard>
            ))}
          </div>
        </div>

        {/* Perspectives */}
        <div>
          <h2 className="text-2xl font-bold mb-4 text-slate-900 dark:text-white">
            Linguistic Perspectives ({perspectives.length})
          </h2>
          <p className="text-sm text-slate-600 dark:text-slate-400 mb-6">
            Pure language localization - native speaker perspectives from 12 different languages,
            each bringing unique linguistic and cultural approaches to thinking.
          </p>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {perspectives.map((perspective) => (
              <GlassCard key={perspective.id} className="p-6 hover:scale-105 transition-transform">
                <div className="flex items-start gap-3 mb-3">
                  <div className="text-3xl">{perspective.emoji}</div>
                  <div className="flex-1">
                    <h3 className="text-lg font-bold text-slate-900 dark:text-white mb-1">
                      {perspective.name}
                    </h3>
                    <p className="text-sm text-slate-600 dark:text-slate-400">
                      {perspective.description}
                    </p>
                  </div>
                </div>
              </GlassCard>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
