# File Size Management Guide

This guide provides a systematic approach for implementing the file size management guidelines established in the `.clinerules` file. It outlines a practical methodology for identifying, analyzing, and splitting large documentation files to ensure optimal performance across all AI models.

## File Size Identification Process

1. **Identify Candidate Files**:
   - Core documentation files (e.g., `systemPatterns.md`, `techContext.md`)
   - Documentation for complex crates (e.g., `router/overview.md`)
   - Any file that has grown through multiple updates

2. **Size Assessment**:
   - Check line count (>300 lines is the threshold for splitting)
   - Examine file structure and section sizes
   - Identify files with sections exceeding 100 lines

## Splitting Strategy Selection

Choose the most appropriate splitting strategy based on the file's content and structure:

### 1. Topic-Based Splitting

**Best for**: Files covering multiple distinct topics, like `techContext.md`

**Implementation**:
- Create a dedicated subdirectory for the topic
- Create an overview/index file that summarizes each topic and provides links
- Split each major topic into its own file
- Ensure each file has a breadcrumb link back to the overview

**Example** (for `techContext.md`):
```
/memory-bank/thematic/technical_environment/
├── overview.md                  # Summary and links to detailed topics
├── technology_stack.md          # Programming language, libraries, database 
├── project_structure.md         # Workspace organization and key crates
├── development_environment.md   # Local setup, configuration, testing
├── deployment_options.md        # Docker, Kubernetes deployment
└── security_and_versioning.md   # Feature flags, security considerations
```

### 2. Hierarchical Splitting

**Best for**: Files with a clear parent-child structure, like `systemPatterns.md`

**Implementation**:
- Create a dedicated subdirectory
- Maintain the high-level overview in the parent document
- Move detailed subsections to child documents
- Link from parent to children for full details

**Example** (for `systemPatterns.md`):
```
/memory-bank/thematic/architecture/
├── overview.md              # High-level architecture and links
├── core_components.md       # Router, Scheduler, etc. details
├── design_patterns.md       # Flow patterns, connector integration
└── cross_cutting_concerns.md # Security, performance, etc.
```

### 3. Temporal Splitting

**Best for**: Files with historical/archival content mixed with current information

**Implementation**:
- Separate current/active information from historical content
- Move historical content to the archive directory
- Maintain links between current and archival content

**Example**:
```
/memory-bank/thematic/development/
├── current_practices.md     # Current development practices
└── /archive/
    └── deprecated_methods.md # Historical methods no longer in use
```

## Document Restructuring Process

1. **Create Directory Structure**:
   ```bash
   mkdir -p /Users/arunraj/github/hyperswitch/memory-bank/thematic/TOPIC_DIRECTORY
   ```

2. **Create Overview File**:
   - Include summary of all topics
   - Provide clear links to detailed files
   - Maintain the same h1 title as the original file for consistency

3. **Extract Content into Topic Files**:
   - Maintain consistent heading structure
   - Ensure each file is independently readable
   - Include cross-references to related topics

4. **Add Metadata Headers**:
   ```markdown
   ---
   parent: Overview
   last_updated: 2025-05-20
   position: 2
   related_files:
     - related_topic.md
   ---
   ```

5. **Update References**:
   - Update any links in other files that pointed to the original file
   - Add notes in the original location if necessary

## Cross-Reference System Implementation

1. **Overview TOC**:
   ```markdown
   ## Contents
   
   - [Technology Stack](./technology_stack.md) - Programming language, libraries, and frameworks
   - [Project Structure](./project_structure.md) - Workspace organization and key crates
   ```

2. **Breadcrumb Navigation**:
   ```markdown
   [← Back to Technical Overview](./overview.md)
   ```

3. **See Also Sections**:
   ```markdown
   ## See Also
   
   - [Development Environment](./development_environment.md) - For local setup details
   - [Deployment Options](./deployment_options.md) - For production deployment information
   ```

## Practice Examples

### Example 1: Splitting techContext.md

1. Create directory structure:
   ```bash
   mkdir -p /Users/arunraj/github/hyperswitch/memory-bank/thematic/technical_environment
   ```

2. Create overview.md:
   ```markdown
   # Hyperswitch Technical Context
   
   This document provides an overview of the technical aspects of Hyperswitch, including technology stack, project structure, development environment, and deployment options.
   
   ## Contents
   
   - [Technology Stack](./technology_stack.md) - Programming language, libraries, and database details
   - [Project Structure](./project_structure.md) - Workspace organization and key crates
   - [Development Environment](./development_environment.md) - Local setup, configuration, testing
   - [Deployment Options](./deployment_options.md) - Docker, Kubernetes deployment
   - [Security and Versioning](./security_and_versioning.md) - Feature flags, security considerations
   ```

3. Create individual topic files with breadcrumb navigation and appropriate content.

4. Update any references to the original techContext.md in other files.

## Quality Checklist

Before finalizing a file split, verify:

- [ ] Each file is independently readable and coherent
- [ ] All cross-references are working correctly
- [ ] No content has been lost during the splitting process
- [ ] Consistent formatting and structure are maintained
- [ ] Overview file provides clear navigation to all content
- [ ] File sizes are now within guidelines (<300 lines each)
- [ ] External references to the original file have been updated

## Implementation Status

Track the status of file size management implementation here:

| File | Status | Split Location | Date Completed |
|------|--------|----------------|----------------|
| systemPatterns.md | Analyzed - Not Required | N/A | 2025-05-20 |
| techContext.md | Analyzed - Not Required | N/A | 2025-05-20 |
| router/overview.md | Completed (Proactive Split) | /memory-bank/thematic/crates/router/ | 2025-05-20 |

*Note: Both systemPatterns.md and techContext.md were analyzed and found to be within our size guidelines, but they provide excellent case studies for our splitting methodology if they were to grow larger in the future.*

### Completed Implementations

#### Router Overview Split

The `router/overview.md` file was proactively split following the hierarchical splitting pattern:

- Created a streamlined overview file with links to detailed documentation
- Split content into logical sections:
  - `/modules/` - Core module details, routes, services, middleware
  - `/flows/` - Payment, refund, and webhook flows
  - `/architecture/` - Code structure, entry points, dependencies
  - `/configuration/` - Feature flags, routing strategies

This implementation allows for easier maintenance and better organization of the documentation, preventing the file from exceeding the 300-line threshold as the documentation grows.
