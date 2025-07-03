# Documentation Review Criteria

## Overview

This document establishes the criteria used to evaluate Memory Bank documentation during the review process. These criteria serve as the foundation for ensuring high-quality, consistent, and useful documentation across the entire Memory Bank.

## Core Evaluation Areas

All Memory Bank documentation is evaluated across five core areas:

1. **Technical Accuracy**
2. **Completeness**
3. **Clarity & Readability**
4. **Structure & Organization**
5. **Consistency & Style**

Each area has specific criteria that must be met for documentation to be approved.

## 1. Technical Accuracy

Technical accuracy is the most critical aspect of documentation review. Documentation must accurately reflect the current system implementation, architecture, and behavior.

### Evaluation Criteria

| Criterion | Description | Priority |
|-----------|-------------|----------|
| Factual correctness | All technical statements, descriptions, and explanations are factually correct | Critical |
| Code accuracy | Code examples, snippets, and references match the actual codebase | Critical |
| API correctness | API descriptions, parameters, and return values are accurate | Critical |
| Architectural accuracy | System architecture descriptions accurately reflect the current implementation | Critical |
| Version accuracy | Content is accurate for the specified version | High |
| Current implementation | Documentation reflects the current implementation, not planned or deprecated features (unless explicitly noted) | High |

### Evaluation Questions

- Are all technical details correct and verified against the codebase?
- Do code examples actually work as described?
- Are API descriptions consistent with the actual implementation?
- Does the architectural documentation accurately represent the system?
- Is the documentation clear about which version it applies to?

## 2. Completeness

Documentation must be comprehensive, covering all necessary information without gaps that would impede understanding.

### Evaluation Criteria

| Criterion | Description | Priority |
|-----------|-------------|----------|
| Scope coverage | Documentation covers the entire scope of the subject matter | High |
| Required sections | All required sections for the document type are present and complete | High |
| Feature coverage | All relevant features and functionality are documented | High |
| Interface documentation | All public interfaces, methods, and parameters are documented | High |
| Error handling | Error conditions, messages, and handling procedures are documented | Medium |
| Edge cases | Common edge cases and their handling are documented | Medium |
| Cross-references | Appropriate references to related documentation are included | Medium |

### Evaluation Questions

- Does the documentation cover all aspects of the subject matter?
- Are there any gaps in the information provided?
- Are all required sections present and sufficiently detailed?
- Are error scenarios and edge cases addressed?
- Are there appropriate links to related documents?

## 3. Clarity & Readability

Documentation must be clear, understandable, and accessible to its intended audience.

### Evaluation Criteria

| Criterion | Description | Priority |
|-----------|-------------|----------|
| Audience appropriateness | Content is appropriate for the intended audience | High |
| Clear explanations | Concepts are explained clearly and logically | High |
| Jargon & terminology | Technical terms are either explained or linked to explanations | Medium |
| Conciseness | Information is presented concisely without unnecessary verbosity | Medium |
| Language quality | Proper grammar, spelling, and punctuation | Medium |
| Readability | Text is readable, with appropriate sentence and paragraph length | Medium |
| Visual aids | Diagrams, charts, and other visual elements enhance understanding where appropriate | Medium |

### Evaluation Questions

- Is the documentation understandable to the intended audience?
- Are complex concepts explained clearly?
- Is technical terminology appropriately explained or referenced?
- Is the writing concise and to the point?
- Is the text free of grammatical and spelling errors?
- Do visual elements effectively support the text?

## 4. Structure & Organization

Documentation must be well-structured and organized to facilitate understanding and navigation.

### Evaluation Criteria

| Criterion | Description | Priority |
|-----------|-------------|----------|
| Logical organization | Content follows a logical progression | High |
| Navigability | Document is easy to navigate with appropriate headings and sections | High |
| Heading structure | Heading hierarchy is appropriate and consistent | Medium |
| Section balance | Sections are appropriately sized and balanced | Medium |
| Information hierarchy | Important information is emphasized and easily discoverable | Medium |
| File size | Document complies with size guidelines (under 300 lines) | Medium |

### Evaluation Questions

- Does the document follow a logical structure?
- Is the hierarchy of information clear and appropriate?
- Are headings and sections used effectively to organize content?
- Can readers easily find specific information within the document?
- Does the document comply with size guidelines?

## 5. Consistency & Style

Documentation must adhere to established style guidelines and maintain consistency across the Memory Bank.

### Evaluation Criteria

| Criterion | Description | Priority |
|-----------|-------------|----------|
| Style guide adherence | Content adheres to the Memory Bank style guide | High |
| Formatting consistency | Formatting is consistent throughout the document | Medium |
| Terminology consistency | Terms are used consistently throughout the document and across related documents | High |
| Template usage | Appropriate template is used and followed | High |
| Voice and tone | Consistent voice and tone throughout the document | Medium |
| Visual consistency | Visual elements follow established patterns and styles | Medium |

### Evaluation Questions

- Does the document adhere to the Memory Bank style guide?
- Is formatting consistent throughout?
- Are terms used consistently?
- Does the document use and follow the appropriate template?
- Is the voice and tone consistent and appropriate?

## Document Type-Specific Criteria

In addition to the core criteria, specific document types have additional evaluation criteria:

### Crate Documentation

| Criterion | Description | Priority |
|-----------|-------------|----------|
| Purpose explanation | Clear explanation of the crate's purpose and role | Critical |
| Interface documentation | Complete documentation of public interfaces | Critical |
| Integration information | Information on how the crate integrates with other components | High |
| Example usage | Examples of common usage patterns | High |
| Architecture description | Description of internal architecture and design patterns | Medium |

### Flow Documentation

| Criterion | Description | Priority |
|-----------|-------------|----------|
| Flow completeness | All steps in the flow are documented | Critical |
| Component interactions | Interactions between components are clearly described | Critical |
| Error paths | Error paths and exception handling are documented | High |
| Sequence diagrams | Flow includes appropriate sequence diagrams | High |
| Entry/exit points | Clear documentation of entry and exit points | Medium |

### API Documentation

| Criterion | Description | Priority |
|-----------|-------------|----------|
| Endpoint completeness | All endpoints are documented | Critical |
| Parameter documentation | All parameters are documented with types and constraints | Critical |
| Response formats | Response formats and status codes are documented | Critical |
| Authentication requirements | Authentication and authorization requirements are clear | High |
| Example requests/responses | Examples of requests and responses are provided | High |

### Configuration Documentation

| Criterion | Description | Priority |
|-----------|-------------|----------|
| Configuration options | All configuration options are documented | Critical |
| Default values | Default values are specified | High |
| Constraints | Constraints and validation rules are documented | High |
| Example configurations | Example configurations for common scenarios | Medium |
| Environment variables | Related environment variables are documented | Medium |

## Severity Levels

During review, issues are categorized by severity:

1. **Critical**: Must be fixed before approval. Documentation is incorrect or missing essential information.
2. **Major**: Should be fixed before approval. Documentation has significant issues that impact usability.
3. **Minor**: Can be approved with these issues, but should be fixed in a future update. Issues are not significant enough to block approval.
4. **Suggestion**: Recommended improvements that are optional.

## Related Documents

- [Review Workflow](01_review_workflow.md)
- [Review Checklists](03_review_checklists.md)
- [Roles and Responsibilities](04_roles_and_responsibilities.md)
- [Feedback Incorporation Process](05_feedback_incorporation.md)
