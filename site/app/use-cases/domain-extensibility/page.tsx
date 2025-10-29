import Link from 'next/link';

export default function DomainExtensibilityPage() {
  return (
    <div className="max-w-4xl mx-auto px-4 py-12">
      {/* Hero Section */}
      <h1 className="text-4xl font-bold mb-4 text-slate-900 dark:text-white">
        Domain Extensibility: AI Generating FinOps Expertise from Research
      </h1>
      <p className="text-lg text-slate-600 dark:text-slate-300 mb-12">
        Watch AI study your UC1 knowledge, create domain roles/workflows, then produce $323K ROI analysis
      </p>

      {/* Prerequisites */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Prerequisites</h2>
        <ul className="list-disc list-inside space-y-2 text-slate-700 dark:text-slate-300">
          <li>Completed <Link href="/use-cases/knowledge-foundation" className="text-blue-600 dark:text-blue-400 hover:underline">UC1</Link> (knowledge foundation about FinOps Framework)</li>
          <li>Claude Code or GitHub Copilot with maenifold MCP</li>
          <li>Access to FinOps Hubs (optional, for live data)</li>
        </ul>
      </section>

      {/* Setup Steps */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Setup</h2>
        <div className="space-y-3">
          <div className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">1.</span>
            <p className="text-slate-700 dark:text-slate-300">Ensure UC1 research complete: <code className="bg-slate-200 dark:bg-slate-800 px-2 py-1 rounded text-sm">memory://research/finops-framework/</code> exists</p>
          </div>
          <div className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">2.</span>
            <p className="text-slate-700 dark:text-slate-300">Built-in workflows available: role-creation-workflow, higher-order-thinking</p>
          </div>
          <div className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">3.</span>
            <p className="text-slate-700 dark:text-slate-300">FinOps Toolkit help files and DB schema (if working with live data)</p>
          </div>
          <div className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">4.</span>
            <p className="text-slate-700 dark:text-slate-300">maenifold configured as MCP server</p>
          </div>
        </div>
      </section>

      {/* Walkthrough Examples */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Walkthrough Examples</h2>

        {/* Example 1 */}
        <div className="mb-12">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">Example 1: AI Creates Domain Roles from UC1 Knowledge</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <p className="text-sm text-slate-600 dark:text-slate-400 mb-4">
              <strong>Scenario:</strong> AI studies FinOps Framework knowledge from UC1 and creates 3 specialist roles.
            </p>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Claude Code Prompt:</strong></p>
            <p className="text-sm font-mono text-slate-900 dark:text-slate-100 mb-4">
              "Use role-creation-workflow. Study my memory://research/finops-framework/ knowledge and the FinOps Foundation website to create three roles: finops-practitioner, cfo, and ftk-agent."
            </p>

            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3"><strong>What AI Does</strong> (role-creation-workflow):</p>
            <ol className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-decimal list-inside mb-4">
              <li>Assumes prompt-engineer role (10/10 constitutional AI expert)</li>
              <li>Defines role specifications from FinOps Framework</li>
              <li>Researches domain knowledge (reads UC1 memory files + FinOps.org)</li>
              <li>Analyzes constitutional requirements (principles, anti-patterns)</li>
              <li>Designs cognitive architecture (workflows, evaluation criteria)</li>
              <li>Creates role structure (JSON with motto, principles, approach)</li>
              <li>Validates against prompt engineering checklists</li>
              <li>Saves to assets/roles/:
                <ul className="ml-6 mt-1 space-y-1 list-disc list-inside text-xs">
                  <li>finops-practitioner.json (FinOps Framework expert)</li>
                  <li>cfo.json (Executive financial stewardship)</li>
                  <li>ftk-agent.json (KQL query executor for FinOps Hubs)</li>
                </ul>
              </li>
            </ol>

            <div className="border-t border-slate-300 dark:border-slate-600 pt-4 mt-4">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Output:</strong></p>
              <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside mb-3">
                <li>3 production-ready role definitions</li>
                <li>Each role studies FinOps Framework knowledge from UC1</li>
                <li>AI generated domain expertise from external docs</li>
              </ul>
              <p className="text-sm text-blue-600 dark:text-blue-400">
                <strong>Why This Matters:</strong> AI creates its own domain knowledge by studying, not pre-programming.
              </p>
            </div>
          </div>
        </div>

        {/* Example 2 */}
        <div className="mb-12">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">Example 2: AI Designs Domain Workflows Using New Roles</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <p className="text-sm text-slate-600 dark:text-slate-400 mb-4">
              <strong>Scenario:</strong> AI adopts finops-practitioner role to design analysis workflows.
            </p>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Claude Code Prompt:</strong></p>
            <p className="text-sm font-mono text-slate-900 dark:text-slate-100 mb-4">
              "Adopt finops-practitioner role. Use higher-order-thinking workflow to design ftk-query and ftk-analysis workflows for Azure cost analysis."
            </p>

            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3"><strong>What AI Does:</strong></p>
            <ol className="text-sm text-slate-700 dark:text-slate-300 space-y-2 list-decimal list-inside mb-4">
              <li>Adopts finops-practitioner (FinOps Framework expertise)</li>
              <li>Uses higher-order-thinking workflow:
                <ul className="ml-6 mt-1 space-y-1 list-disc list-inside text-xs">
                  <li>Examines thinking processes for workflow design</li>
                  <li>Evaluates cognitive approaches</li>
                  <li>Synthesizes multiple perspectives</li>
                  <li>Designs ftk-query (data collection) and ftk-analysis (strategic reporting)</li>
                </ul>
              </li>
              <li>Then adopts ftk-agent role to add correct KQL queries to workflows</li>
              <li>Saves to assets/workflows/:
                <ul className="ml-6 mt-1 space-y-1 list-disc list-inside text-xs">
                  <li>ftk-query.json (FinOps data collection & optimization)</li>
                  <li>ftk-analysis.json (FinOps strategic analysis & reporting)</li>
                </ul>
              </li>
            </ol>

            <div className="border-t border-slate-300 dark:border-slate-600 pt-4 mt-4">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Output:</strong></p>
              <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside mb-3">
                <li>Domain-specific workflows designed by AI</li>
                <li>Query patterns from ftk-agent role</li>
                <li>Strategic frameworks from finops-practitioner role</li>
              </ul>
              <p className="text-sm text-blue-600 dark:text-blue-400">
                <strong>Why This Matters:</strong> AI designs workflows by adopting domain expertise roles it created.
              </p>
            </div>
          </div>
        </div>

        {/* Example 3 */}
        <div className="mb-12">
          <h3 className="text-xl font-semibold mb-4 text-slate-900 dark:text-white">Example 3: Execute Workflows → Produce $323K ROI Report</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
            <p className="text-sm text-slate-600 dark:text-slate-400 mb-4">
              <strong>Scenario:</strong> Run ftk-analysis workflow using all 3 roles to produce executive report.
            </p>

            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3"><strong>Process:</strong></p>
            <div className="space-y-3 mb-4">
              <div className="text-sm">
                <p className="text-slate-900 dark:text-white font-semibold mb-1">1. ftk-agent executes KQL queries against FinOps Hubs</p>
                <ul className="text-xs text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
                  <li>Collects <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[cost-data]]</code>, <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[commitments]]</code>, <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[anomalies]]</code></li>
                  <li>Stores results in memory:// with [[WikiLinks]]</li>
                </ul>
              </div>
              <div className="text-sm">
                <p className="text-slate-900 dark:text-white font-semibold mb-1">2. finops-practitioner analyzes findings</p>
                <ul className="text-xs text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
                  <li>Applies <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[FinOps-Framework]]</code> best practices</li>
                  <li>Identifies <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[optimization-opportunities]]</code></li>
                  <li>Calculates <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[ROI]]</code> and <code className="bg-slate-200 dark:bg-slate-800 px-1 py-0.5 rounded">[[payback-period]]</code></li>
                </ul>
              </div>
              <div className="text-sm">
                <p className="text-slate-900 dark:text-white font-semibold mb-1">3. cfo synthesizes executive report</p>
                <ul className="text-xs text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
                  <li>Strategic context for board presentation</li>
                  <li>Risk assessment and mitigation</li>
                  <li>Multi-scenario financial projections</li>
                </ul>
              </div>
            </div>

            <div className="border-t border-slate-300 dark:border-slate-600 pt-4 mt-4">
              <p className="text-sm text-slate-700 dark:text-slate-300 mb-2"><strong>Output</strong> (SONNET-A Report):</p>
              <div className="grid grid-cols-2 gap-3 text-sm text-slate-700 dark:text-slate-300 mb-3">
                <div><strong>Annual Savings:</strong> $323,875</div>
                <div><strong>Implementation Cost:</strong> $50,700</div>
                <div><strong>ROI:</strong> 638%</div>
                <div><strong>Payback Period:</strong> 1.9 months</div>
                <div><strong>Current State:</strong> Grade D+ (5th percentile)</div>
                <div><strong>Target State:</strong> 85th percentile, B+ maturity</div>
              </div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Strategic Roadmap: 18-month transformation to B+ FinOps maturity</p>
              <p className="text-sm text-blue-600 dark:text-blue-400 mt-3">
                <strong>Why This Matters:</strong> $323K savings identified → ROI on setup time achieved immediately.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* The Meta-Capability */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">The Meta-Capability</h2>
        <div className="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-6 border border-blue-200 dark:border-blue-800">
          <p className="text-slate-700 dark:text-slate-300 mb-4">
            This isn't about pre-built FinOps roles. It's about:
          </p>
          <ol className="text-slate-700 dark:text-slate-300 space-y-2 list-decimal list-inside mb-4">
            <li><strong>UC1:</strong> AI researches FinOps Framework → builds knowledge graph</li>
            <li><strong>UC2:</strong> AI studies that knowledge → creates domain roles</li>
            <li><strong>UC2:</strong> AI adopts roles → designs workflows</li>
            <li><strong>UC2:</strong> AI executes workflows → produces $323K ROI analysis</li>
          </ol>
          <p className="text-blue-600 dark:text-blue-400 font-semibold">
            The Innovation: AI generating domain expertise on demand by studying external docs.
          </p>
        </div>
      </section>

      {/* Demo Artifacts */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Demo Artifacts - Available for Inspection</h2>
        <div className="space-y-6">
          <div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">Real FinOps Analysis Outputs (SONNET-A folder)</h3>
            <p className="text-sm text-slate-700 dark:text-slate-300 mb-3">Available on website for inspection:</p>
            <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
              <li>finops-strategic-report.md (367 lines, executive analysis)</li>
              <li>executive-summary.md (business impact)</li>
              <li>implementation-roadmap.md (18-month transformation plan)</li>
              <li>recommendations.md (actionable optimizations)</li>
              <li>roi-analysis.json (financial projections)</li>
              <li>20+ JSON data files (cost breakdowns, forecasts, anomalies)</li>
            </ul>
          </div>

          <div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">Domain Roles</h3>
            <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
              <li>finops-practitioner.json (FinOps Framework principles)</li>
              <li>cfo.json (Executive financial stewardship)</li>
              <li>ftk-agent.json (KQL query executor)</li>
            </ul>
          </div>

          <div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">Domain Workflows</h3>
            <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
              <li>ftk-query.json (Data collection methodology)</li>
              <li>ftk-analysis.json (Strategic reporting workflow)</li>
            </ul>
          </div>
        </div>
      </section>

      {/* Code Sample */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Code Sample</h2>
        <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-6 border border-slate-200 dark:border-slate-700">
          <pre className="text-sm font-mono text-slate-900 dark:text-slate-100 overflow-x-auto">
            <code>{`# Create roles from research knowledge
maenifold --tool Workflow --payload '{
  "workflowId": "role-creation-workflow",
  "response": "Create finops-practitioner role from memory://research/finops-framework/"
}'

# Design workflows using new roles
maenifold --tool Workflow --payload '{
  "workflowId": "higher-order-thinking",
  "response": "Design ftk-analysis workflow for Azure cost optimization"
}'

# Execute analysis (requires FinOps Hub access)
maenifold --tool Workflow --payload '{
  "workflowId": "ftk-analysis",
  "response": "Analyze Azure costs for fiscal year 2025"
}'`}</code>
          </pre>
        </div>
      </section>

      {/* ROI Calculation */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">ROI Calculation</h2>
        <div className="bg-green-50 dark:bg-green-900/20 rounded-lg p-6 border border-green-200 dark:border-green-800">
          <div className="mb-4">
            <p className="text-slate-900 dark:text-white font-semibold mb-2">Setup Investment:</p>
            <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4">
              <li>UC1 Research: 60-90 minutes (one-time)</li>
              <li>Role Creation: 60-90 minutes (3 roles, one-time)</li>
              <li>Workflow Design: 30-45 minutes (one-time)</li>
            </ul>
            <p className="text-sm text-slate-600 dark:text-slate-400 mt-2"><strong>Total Setup:</strong> ~3 hours one-time investment</p>
          </div>

          <div className="border-t border-green-300 dark:border-green-700 pt-4">
            <p className="text-slate-900 dark:text-white font-semibold mb-2">Output Value:</p>
            <ul className="text-sm text-slate-700 dark:text-slate-300 space-y-1 list-disc list-inside ml-4 mb-3">
              <li>$323K annual savings identified</li>
              <li>638% ROI</li>
              <li>1.9-month payback period</li>
              <li>Ongoing analysis capability</li>
            </ul>
            <p className="text-green-600 dark:text-green-400 font-semibold">
              Why Should I Care? Configure once, get ongoing multi-perspective financial analysis forever. Suddenly you have analyst + practitioner + CFO analyzing your spend.
            </p>
          </div>
        </div>
      </section>

      {/* Common Pitfalls */}
      <section className="mb-16">
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Common Pitfalls</h2>
        <div className="space-y-3 text-slate-700 dark:text-slate-300">
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>UC1 prerequisite:</strong> Must complete UC1 research first—roles need knowledge foundation
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Role creation time:</strong> Expect 20-30 minutes per role for proper constitutional design
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Workflow design:</strong> higher-order-thinking requires systematic reasoning, not quick prompts
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>FinOps Hub access:</strong> Live analysis requires Azure MCP Kusto Tools connection
            </div>
          </div>
          <div className="flex gap-3">
            <span>⚠️</span>
            <div>
              <strong>Asset modification:</strong> Roles/workflows saved to ~/maenifold/assets/ (user-modifiable)
            </div>
          </div>
        </div>
      </section>

      {/* Next Steps */}
      <section>
        <h2 className="text-2xl font-bold mb-6 text-slate-900 dark:text-white">Next Steps</h2>
        <ul className="space-y-3">
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <Link href="/use-cases/dev-work" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                UC3: Single-Agent Dev Workflows
              </Link>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Benefit without multi-agent complexity
              </p>
            </div>
          </li>
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <p className="text-slate-700 dark:text-slate-300 font-semibold">Try applying this pattern to different domains</p>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Security, compliance, architecture—any domain you can research
              </p>
            </div>
          </li>
          <li className="flex gap-3">
            <span className="text-blue-500 dark:text-blue-400 font-bold">→</span>
            <div>
              <Link href="/tools" className="text-blue-600 dark:text-blue-400 hover:underline font-semibold">
                Explore Tools & Workflows
              </Link>
              <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
                Browse workflow tools and memory management capabilities
              </p>
            </div>
          </li>
        </ul>
      </section>
    </div>
  );
}
