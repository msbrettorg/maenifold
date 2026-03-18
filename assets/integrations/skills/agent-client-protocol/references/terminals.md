<!-- Minimal local copy of the official ACP docs. Source: https://agentclientprotocol.com/protocol/terminals -->

# Terminals

> Executing and managing terminal commands

The terminal methods allow Agents to execute shell commands within the Client's environment. These methods enable Agents to run build processes, execute scripts, and interact with command-line tools while providing real-time output streaming and process control.

## Checking Support

Before attempting to use terminal methods, Agents **MUST** verify that the Client supports this capability by checking the Client Capabilities field in the `initialize` response.

If `terminal` is `false` or not present, the Agent **MUST NOT** attempt to call any terminal methods.

## Executing Commands

The `terminal/create` method starts a command in a new terminal.

The Client returns a Terminal ID immediately without waiting for completion.

This allows the command to run in the background while the Agent performs other operations.

After creating the terminal, the Agent can use the `terminal/wait_for_exit` method to wait for the command to complete.

<Note>
  The Agent **MUST** release the terminal using `terminal/release` when it's no
  longer needed.
</Note>

## Embedding in Tool Calls

Terminals can be embedded directly in tool calls to provide real-time output to users.

When a terminal is embedded in a tool call, the Client displays live output as it's generated and continues to display it even after the terminal is released.

## Getting Output

The `terminal/output` method retrieves the current terminal output without waiting for the command to complete.

## Waiting for Exit

The `terminal/wait_for_exit` method returns once the command completes.

## Killing Commands

The `terminal/kill` method terminates a command without releasing the terminal.

After killing a command, the terminal remains valid and can be used with:

* `terminal/output` to get the final output
* `terminal/wait_for_exit` to get the exit status

The Agent **MUST** still call `terminal/release` when it's done using it.

### Building a Timeout

Agents can implement command timeouts by combining terminal methods:

1. Create a terminal with `terminal/create`
2. Start a timer for the desired timeout duration
3. Concurrently wait for either the timer to expire or `terminal/wait_for_exit` to return
4. If the timer expires first:
   * Call `terminal/kill` to terminate the command
   * Call `terminal/output` to retrieve any final output
   * Include the output in the response to the model
5. Call `terminal/release` when done

## Releasing Terminals

The `terminal/release` kills the command if still running and releases all resources.

After release the terminal ID becomes invalid for all other `terminal/*` methods.
