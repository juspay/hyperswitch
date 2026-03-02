# Hyperswitch Diagram Style Guide

This document defines the visual standards for all diagrams in Hyperswitch documentation. Consistent styling ensures professional, readable, and accessible diagrams across all documentation.

## 1. Color Palette

### Primary Colors

| Color Name | Hex Code | Usage |
|------------|----------|-------|
| Primary Blue | `#1A73E8` | Titles, primary actions, key highlights |
| Dark Text | `#333333` | Body text, labels, descriptions |
| White | `#FFFFFF` | Backgrounds, text on dark elements |

### Semantic Colors

| Category | Primary Color | Light Fill | Usage |
|----------|--------------|------------|-------|
| **Experience/Merchant Layer** | `#4CAF50` | `#C5E8C0` | Merchant-facing components, user experience layer |
| **Backend/Connector** | `#5B9BD5` / `#6CB4EE` | `#B3D9F2` | Backend services, connector implementations |
| **External/PSP** | `#F5A623` | `#F9B872` | External payment service providers, third-party systems |
| **Integrations** | `#9B72CF` | `#D8C8E8` | Integration points, webhook handlers, callbacks |
| **DevOps/Tools** | `#4DD0B8` | `#C0F0E8` | Development tools, monitoring, deployment |

### Utility Colors

| Color Name | Hex Code | Usage |
|------------|----------|-------|
| Periwinkle | `#B8B8F0` | Borders, dividers |
| Process Gray | `#E0E0E0` | Process boxes, neutral containers |
| Light Border Gray | `#D0D0D0` | Subtle borders, separators |

### Color Application Rules

1. **Fill vs Border**: Use light fill for container backgrounds, primary color for borders/headers
2. **Contrast**: Ensure text contrast ratio of at least 4.5:1 for accessibility
3. **Consistency**: Same component type should always use the same color across all diagrams
4. **Gradients**: Avoid gradients - use flat, solid colors

## 2. Typography

### Font Family

```css
font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
```

### Font Sizes

| Element | Size | Weight | Line Height |
|---------|------|--------|-------------|
| Diagram Title | 18px | 600 (Semibold) | 1.3 |
| Section Headers | 14px | 600 (Semibold) | 1.4 |
| Component Labels | 12px | 500 (Medium) | 1.4 |
| Body Text | 11px | 400 (Regular) | 1.5 |
| Captions/Notes | 10px | 400 (Regular) | 1.4 |

### Text Styling

- **Alignment**: Center for headers, left-align for body text within containers
- **Color**: `#333333` for primary text, `#666666` for secondary text
- **Transform**: Use sentence case for labels, Title Case for diagram titles

## 3. Shape Specifications

### Rounded Corners

| Element Type | Border Radius |
|--------------|---------------|
| Small boxes (buttons, badges) | 4-6px |
| Medium boxes (components) | 6-8px |
| Large containers (layers, groups) | 8-12px |
| Process boxes | 6px |

### Border Styles

| Element | Border Width | Border Color |
|---------|--------------|--------------|
| Primary containers | 2px | Category primary color |
| Secondary containers | 1.5px | `#D0D0D0` or `#B8B8F0` |
| Highlighted elements | 2px | `#1A73E8` |
| Dashed borders (optional) | 1.5px | `#999999` |

### Container Sizing

| Component Type | Min Width | Min Height | Padding |
|----------------|-----------|------------|---------|
| Small component | 80px | 40px | 8px 12px |
| Medium component | 120px | 50px | 10px 16px |
| Large container | 200px | 80px | 12px 20px |
| Layer container | 300px | 150px | 16px 20px |

## 4. Arrow and Line Styles

### Connection Types

| Type | Style | Usage |
|------|-------|-------|
| **Synchronous** | Solid line, filled arrow | Blocking request/response |
| **Asynchronous** | Solid line, open arrow | Non-blocking operations |
| **Return/Response** | Dashed line, open arrow | Response flows |
| **Data Flow** | Solid line, no arrow | Data movement (bidirectional) |
| **Optional/Conditional** | Dotted line, filled arrow | Conditional paths |

### Arrow Specifications

```svg
<!-- Filled Arrow (Sync) -->
<marker id="arrowFilled" markerWidth="10" markerHeight="10" 
        refX="9" refY="3" orient="auto" markerUnits="strokeWidth">
  <path d="M0,0 L0,6 L9,3 z" fill="#333333"/>
</marker>

<!-- Open Arrow (Async/Return) -->
<marker id="arrowOpen" markerWidth="10" markerHeight="10" 
        refX="9" refY="3" orient="auto" markerUnits="strokeWidth">
  <path d="M0,0 L9,3 L0,6" fill="none" stroke="#333333" stroke-width="1.5"/>
</marker>
```

### Line Weights

| Line Type | Stroke Width |
|-----------|--------------|
| Primary flow | 2px |
| Secondary flow | 1.5px |
| Lifelines (sequence diagrams) | 1px dashed |
| Container borders | 2px |

## 5. SVG Template Code Snippets

### Component Box Template

```svg
<svg width="140" height="60" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <style>
      .component-box { fill: #B3D9F2; stroke: #5B9BD5; stroke-width: 2; rx: 8; }
      .component-text { font-family: Inter, sans-serif; font-size: 12px; 
                        fill: #333333; text-anchor: middle; }
    </style>
  </defs>
  <rect class="component-box" x="1" y="1" width="138" height="58"/>
  <text class="component-text" x="70" y="35">Component Name</text>
</svg>
```

### Layer Container Template

```svg
<svg width="400" height="200" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <style>
      .layer-container { fill: #C5E8C0; stroke: #4CAF50; stroke-width: 2; rx: 12; }
      .layer-header { fill: #4CAF50; }
      .layer-title { font-family: Inter, sans-serif; font-size: 14px; 
                     fill: #FFFFFF; font-weight: 600; }
    </style>
  </defs>
  <!-- Container body -->
  <rect class="layer-container" x="1" y="1" width="398" height="198"/>
  <!-- Header bar -->
  <rect class="layer-header" x="1" y="1" width="398" height="30" rx="12"/>
  <!-- Clip header bottom corners -->
  <rect fill="#4CAF50" x="1" y="15" width="398" height="16"/>
  <text class="layer-title" x="200" y="22">Experience Layer</text>
</svg>
```

### Sequence Diagram Lifeline

```svg
<svg width="100" height="300" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <style>
      .participant-box { fill: #B3D9F2; stroke: #5B9BD5; stroke-width: 2; rx: 6; }
      .lifeline { stroke: #999999; stroke-width: 1; stroke-dasharray: 5,5; }
      .activation { fill: #5B9BD5; stroke: #5B9BD5; stroke-width: 1; rx: 2; }
    </style>
  </defs>
  <!-- Participant header -->
  <rect class="participant-box" x="10" y="10" width="80" height="40"/>
  <text x="50" y="35" font-family="Inter" font-size="11" 
        fill="#333333" text-anchor="middle">Service</text>
  <!-- Lifeline -->
  <line class="lifeline" x1="50" y1="50" x2="50" y2="280"/>
  <!-- Activation bar -->
  <rect class="activation" x="42" y="80" width="16" height="60"/>
</svg>
```

### Flow Arrow Templates

```svg
<!-- Synchronous Request Arrow -->
<svg width="200" height="40" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <marker id="syncArrow" markerWidth="10" markerHeight="10" 
            refX="9" refY="3" orient="auto">
      <path d="M0,0 L0,6 L9,3 z" fill="#333333"/>
    </marker>
  </defs>
  <line x1="10" y1="20" x2="180" y2="20" 
        stroke="#333333" stroke-width="2" marker-end="url(#syncArrow)"/>
  <text x="100" y="15" font-family="Inter" font-size="10" 
        fill="#666666" text-anchor="middle">request()</text>
</svg>

<!-- Response Arrow (Dashed) -->
<svg width="200" height="40" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <marker id="responseArrow" markerWidth="10" markerHeight="10" 
            refX="9" refY="3" orient="auto">
      <path d="M0,0 L9,3 L0,6" fill="none" stroke="#333333" stroke-width="1.5"/>
    </marker>
  </defs>
  <line x1="180" y1="20" x2="10" y2="20" 
        stroke="#333333" stroke-width="1.5" stroke-dasharray="6,3" 
        marker-end="url(#responseArrow)"/>
  <text x="100" y="15" font-family="Inter" font-size="10" 
        fill="#666666" text-anchor="middle">response</text>
</svg>
```

## 6. Icon and Logo Guidelines

### Logo Placement

- Position logos in the top-left corner of component boxes
- Maintain 8px padding from edges
- Max logo height: 20px for medium components, 28px for large containers

### Logo Format

- Prefer SVG format for scalability
- Use monochrome versions when possible
- Ensure brand colors don't conflict with diagram palette

### Placeholder Format

When actual logos aren't available, use text initials in a colored circle:

```svg
<svg width="24" height="24" xmlns="http://www.w3.org/2000/svg">
  <circle cx="12" cy="12" r="11" fill="#5B9BD5"/>
  <text x="12" y="16" font-family="Inter" font-size="10" 
        fill="#FFFFFF" text-anchor="middle" font-weight="600">HS</text>
</svg>
```

## 7. Accessibility Considerations

### Color Blindness

- Don't rely solely on color to convey meaning
- Use patterns, labels, or icons as additional indicators
- Test diagrams with color blindness simulators

### Text Readability

- Minimum font size: 10px
- Ensure sufficient contrast (4.5:1 minimum)
- Avoid text over complex backgrounds

### Alternative Text

All diagrams should include:
- Descriptive alt text for screen readers
- A text-based description following the diagram
- Mermaid or PlantUML equivalents where applicable

## 8. Export Guidelines

### Supported Formats

| Format | Use Case | Settings |
|--------|----------|----------|
| SVG | Web documentation, scaling | Embedded fonts, optimized paths |
| PNG | Static exports, presentations | 2x scale, transparent background |
| PDF | Print documentation | Vector output, CMYK for print |

### Optimization

- Remove unnecessary metadata
- Simplify paths where possible
- Use CSS classes instead of inline styles
- Minify SVG output for web

## 9. Version Control

All diagram source files should be:
- Stored in version control alongside documentation
- Named descriptively: `diagram-[type]-[component].svg`
- Include creation date and author in file metadata
- Have corresponding text descriptions in documentation

---

## Quick Reference Card

```
Colors:
  Merchant: #4CAF50 / #C5E8C0
  Backend:  #5B9BD5 / #B3D9F2
  External: #F5A623 / #F9B872
  Integrate:#9B72CF / #D8C8E8
  DevOps:   #4DD0B8 / #C0F0E8

Sizing:
  Small corners: 4-6px
  Large corners: 8-12px
  Borders: 1.5-2px

Typography:
  Font: Inter
  Title: 18px semibold
  Labels: 12px medium
  Body: 11px regular
```
