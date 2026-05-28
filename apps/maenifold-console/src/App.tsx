import { Card, CardContent } from '@/components/ui/card'

function App() {
  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-6">
      <Card className="w-full max-w-lg border border-border bg-card">
        <CardContent className="p-6 text-center text-lg text-card-foreground">
          Maenifold Console — bootstrap
        </CardContent>
      </Card>
    </main>
  )
}

export default App
