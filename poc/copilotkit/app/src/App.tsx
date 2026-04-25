import { CopilotKit } from '@copilotkit/react-core';
import { CopilotChat } from '@copilotkit/react-ui';
import '@copilotkit/react-ui/styles.css';

function App() {
  return (
    <div className="app">
      <header className="header">
        <div className="brand">
          <span className="dot" />
          <span className="name">maenifold</span>
          <span className="label">copilotkit poc</span>
        </div>
      </header>
      <main className="chat-container">
        <CopilotKit runtimeUrl="/api" agent="copilot">
          <CopilotChat
            labels={{
              title: 'maenifold',
              initial: 'What would you like to explore?',
              placeholder: 'Ask maenifold anything...',
            }}
            className="copilot-chat"
          />
        </CopilotKit>
      </main>
    </div>
  );
}

export default App;
