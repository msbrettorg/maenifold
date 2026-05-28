import { Card, CardContent } from "@/components/ui/card";
import { BridgeProvider } from "@/lib/bridge/BridgeProvider";

function App() {
  return (
    <BridgeProvider>
      <div className="min-h-screen bg-background text-foreground">
        <main className="flex min-h-screen items-center justify-center px-6">
          <Card className="w-full max-w-lg rounded-lg border border-border bg-card p-8 text-card-foreground">
            <CardContent className="p-0 text-center">
              <h1 className="text-primary text-2xl font-semibold tracking-tight">
                Maenifold Console — bootstrap
              </h1>
            </CardContent>
          </Card>
        </main>
      </div>
    </BridgeProvider>
  );
}

export default App;
