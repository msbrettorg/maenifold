import { TooltipProvider } from "@/components/ui/tooltip";
import { BridgeProvider } from "@/lib/bridge/BridgeProvider";
import PipelineScreen from "@/screens/Pipeline";

function App() {
  return (
    <BridgeProvider>
      <TooltipProvider>
        <PipelineScreen />
      </TooltipProvider>
    </BridgeProvider>
  );
}

export default App;
