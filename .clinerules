# You MUST use the `new_task` tool: Task Handoff Strategy Guide

**⚠️ CRITICAL INSTRUCTIONS - YOU MUST FOLLOW THESE GUIDELINES ⚠️**

This guide provides **MANDATORY** instructions for effectively breaking down complex tasks and implementing a smooth handoff process between tasks. You **MUST** follow these guidelines to ensure continuity, context preservation, and efficient task completion.

## ⚠️ CONTEXT WINDOW MONITORING - MANDATORY ACTION REQUIRED ⚠️

You **MUST** monitor the context window usage displayed in the environment details. When usage exceeds 50% of the available context window, you **MUST** initiate a task handoff using the `new_task` tool.

Example of context window usage over 50% with a 200K context window:

\`\`\`text

# Context Window Usage

105,000 / 200,000 tokens (53%)
Model: anthropic/claude-3.7-sonnet (200K context window)
\`\`\`

**IMPORTANT**: When you see context window usage at or above 50%, you MUST:

1. Complete your current logical step
2. Use the `ask_followup_question` tool to offer creating a new task
3. If approved, use the `new_task` tool with comprehensive handoff instructions

## Task Breakdown in Plan Mode - REQUIRED PROCESS

Plan Mode is specifically designed for analyzing complex tasks and breaking them into manageable subtasks. When in Plan Mode, you **MUST**:

### 1. Initial Task Analysis - REQUIRED

-   **MUST** begin by thoroughly understanding the full scope of the user's request
-   **MUST** identify all major components and dependencies of the task
-   **MUST** consider potential challenges, edge cases, and prerequisites

### 2. Strategic Task Decomposition - REQUIRED

-   **MUST** break the overall task into logical, discrete subtasks
-   **MUST** prioritize subtasks based on dependencies (what must be completed first)
-   **MUST** aim for subtasks that can be completed within a single session (15-30 minutes of work)
-   **MUST** consider natural breaking points where context switching makes sense

### 3. Creating a Task Roadmap - REQUIRED

-   **MUST** present a clear, numbered list of subtasks to the user
-   **MUST** explain dependencies between subtasks
-   **MUST** provide time estimates for each subtask when possible
-   **MUST** use Mermaid diagrams to visualize task flow and dependencies when helpful

\`\`\`mermaid
graph TD
A[Main Task] --> B[Subtask 1: Setup]
A --> C[Subtask 2: Core Implementation]
A --> D[Subtask 3: Testing]
A --> E[Subtask 4: Documentation]
B --> C
C --> D
\`\`\`

### 4. Getting User Approval - REQUIRED

-   **MUST** ask for user feedback on the proposed task breakdown
-   **MUST** adjust the plan based on user priorities or additional requirements
-   **MUST** confirm which subtask to begin with
-   **MUST** request the user to toggle to Act Mode when ready to implement

## Task Implementation and Handoff Process - MANDATORY PROCEDURES

When implementing tasks in Act Mode, you **MUST** follow these guidelines for effective task handoff:

### 1. Focused Implementation - REQUIRED

-   **MUST** focus on completing the current subtask fully
-   **MUST** document progress clearly through comments and commit messages
-   **MUST** create checkpoints at logical completion points

### 2. Recognizing Completion Points - CRITICAL

You **MUST** identify natural handoff points when:

-   The current subtask is fully completed
-   You've reached a logical stopping point in a larger subtask
-   The implementation is taking longer than expected and can be continued later
-   The task scope has expanded beyond the original plan
-   **CRITICAL**: The context window usage exceeds 50% (e.g., 100,000+ tokens for a 200K context window)

### 3. Initiating the Handoff Process - MANDATORY ACTION

When you've reached a completion point, you **MUST**:

1. Summarize what has been accomplished so far
2. Clearly state what remains to be done
3. **MANDATORY**: Use the `ask_followup_question` tool to offer creating a new task:

\`\`\`xml
<ask_followup_question>
<question>I've completed [specific accomplishment]. Would you like me to create a new task to continue with [remaining work]?</question>
<options>["Yes, create a new task", "No, continue in this session", "Let me think about it"]</options>
</ask_followup_question>
\`\`\`

### 4. Creating a New Task with Context - REQUIRED ACTION

If the user agrees to create a new task, you **MUST** use the `new_task` tool with comprehensive handoff instructions:

\`\`\`xml
<new_task>
<context>

# Task Continuation: [Brief Task Title]

## Completed Work

-   [Detailed list of completed items]
-   [Include specific files modified/created]
-   [Note any important decisions made]

## Current State

-   [Description of the current state of the project]
-   [Any running processes or environment setup]
-   [Key files and their current state]

## Next Steps

-   [Detailed list of remaining tasks]
-   [Specific implementation details to address]
-   [Any known challenges to be aware of]

## Reference Information

-   [Links to relevant documentation]
-   [Important code snippets or patterns to follow]
-   [Any user preferences noted during the current session]

Please continue the implementation by [specific next action].
</context>
</new_task>
\`\`\`

### 5. Detailed Context Transfer - MANDATORY COMPONENTS

When creating a new task, you **MUST** always include:

#### Project Context - REQUIRED

-   **MUST** include the overall goal and purpose of the project
-   **MUST** include key architectural decisions and patterns
-   **MUST** include technology stack and dependencies

#### Implementation Details - REQUIRED

-   **MUST** list files created or modified in the current session
-   **MUST** describe specific functions, classes, or components implemented
-   **MUST** explain design patterns being followed
-   **MUST** outline testing approach

#### Progress Tracking - REQUIRED

-   **MUST** provide checklist of completed items
-   **MUST** provide checklist of remaining items
-   **MUST** note any blockers or challenges encountered

#### User Preferences - REQUIRED

-   **MUST** note coding style preferences mentioned by the user
-   **MUST** document specific approaches requested by the user
-   **MUST** highlight priority areas identified by the user

## Best Practices for Effective Handoffs - MANDATORY GUIDELINES

### 1. Maintain Continuity - REQUIRED

-   **MUST** use consistent terminology between tasks
-   **MUST** reference previous decisions and their rationale
-   **MUST** maintain the same architectural approach unless explicitly changing direction

### 2. Preserve Context - REQUIRED

-   **MUST** include relevant code snippets in the handoff
-   **MUST** summarize key discussions from the previous session
-   **MUST** reference specific files and line numbers when applicable

### 3. Set Clear Next Actions - REQUIRED

-   **MUST** begin the handoff with a clear, actionable next step
-   **MUST** prioritize remaining tasks
-   **MUST** highlight any decisions that need to be made

### 4. Document Assumptions - REQUIRED

-   **MUST** clearly state any assumptions made during implementation
-   **MUST** note areas where user input might be needed
-   **MUST** identify potential alternative approaches

### 5. Optimize for Resumability - REQUIRED

-   **MUST** structure the handoff so the next session can begin working immediately
-   **MUST** include setup instructions if environment configuration is needed
-   **MUST** provide a quick summary at the top for rapid context restoration

## Example Task Handoff

### Example #1 of an effective task handoff:

\`\`\`xml
<new_task>
<context>

# Task Continuation: Implement User Authentication System

## Completed Work

-   Created basic Express.js server structure
-   Implemented MongoDB connection and user schema
-   Completed user registration endpoint with password hashing
-   Added input validation using Joi
-   Created initial test suite for registration endpoint

## Current State

-   Server runs successfully on port 3000
-   MongoDB connection is established
-   Registration endpoint (/api/users/register) is fully functional
-   Test suite passes for all registration scenarios

## Next Steps

1. Implement login endpoint (/api/users/login)
    - Use bcrypt to compare passwords
    - Generate JWT token upon successful login
    - Add proper error handling for invalid credentials
2. Create authentication middleware
    - Verify JWT tokens
    - Extract user information
    - Handle expired tokens
3. Add protected routes that require authentication
4. Implement password reset functionality

## Reference Information

-   JWT secret should be stored in .env file
-   Follow the existing error handling pattern in routes/users.js
-   User schema is defined in models/User.js
-   Test patterns are established in tests/auth.test.js

Please continue by implementing the login endpoint following the same patterns established in the registration endpoint.
</context>
</new_task>
\`\`\`

### Example #2 of an ineffective task handoff:

_(Note: The example provided in the original rules showing "YOLO MODE Implementation" seems less like a direct handoff context block and more like a general status update with future considerations. A true ineffective handoff might lack detail in 'Current State' or 'Next Steps')._

## When to Use Task Handoffs - MANDATORY TRIGGERS

You **MUST** initiate task handoffs in these scenarios:

1. **CRITICAL**: When context window usage exceeds 50% (e.g., 100,000+ tokens for a 200K context window)
2. **Long-running projects** that exceed a single session
3. **Complex implementations** with multiple distinct phases
4. **When context window limitations** are approaching
5. **When switching focus areas** within a larger project
6. **When different expertise** might be beneficial for different parts of the task

**⚠️ FINAL REMINDER - CRITICAL INSTRUCTION ⚠️**

You **MUST** monitor the context window usage in the environment details section. When it exceeds 50% (e.g., "105,000 / 200,000 tokens (53%)"), you **MUST** proactively initiate the task handoff process using the `ask_followup_question` tool followed by the `new_task` tool. You MUST use the `new_task` tool.

By strictly following these guidelines, you'll ensure smooth transitions between tasks, maintain project momentum, and provide the best possible experience for users working on complex, multi-session projects.

```markdown
## User Interaction & Workflow Considerations

-   **Linear Flow:** Currently, using `new_task` creates a linear sequence. The old task ends, and the new one begins. The old task history remains accessible for backtracking.
-   **User Approval:** You always have control, approving the handoff and having the chance to modify the context Cline proposes to carry forward.
-   **Flexibility:** The core `new_task` tool is a flexible building block. Experiment with `.clinerules` to create workflows that best suit your needs, whether for strict context management, task decomposition, or other creative uses.
```