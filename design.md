# Comic Reader — Application Specification

**Project type:** Personal desktop comic reader and library manager  
**Initial focus:** Sonic comics, particularly IDW and Archie  
**Platform:** Windows desktop  
**Primary goal:** Create a polished, interactive comic reading experience with an organised library and an interconnected database of comic universes.

---

# 1. Project Overview

The application is a desktop comic reader that combines a traditional comic-reading experience with library management, reading progression, chronological organisation, and an expandable comic-universe database.

It should support importing locally stored comic files, organising them into series and collections, tracking reading progress, and navigating interconnected storylines.

The application should be designed to work locally without requiring an internet connection for normal reading.

## Design principles

- **Reader first:** Reading comics should be effortless, responsive, and visually comfortable.
- **Local ownership:** Comic files and reading data should remain accessible locally.
- **Discoverability:** Users should be able to navigate between series, characters, story arcs, and appearances.
- **Spoiler awareness:** Information should respect the user's reading progress.
- **Extensibility:** The underlying architecture should support future features without requiring a complete rewrite.
- **Visual quality:** The interface should feel like a dedicated media application rather than a generic file manager.

---

# 2. Development Stages

## Stage 1 — Core Application

The foundation of the application. This stage should produce a fully usable comic reader with a complete library management system.

### 2.1 Multiple Reading Modes

Support several ways of displaying comic pages.

Required modes:

- Single-page reading.
- Double-page spreads.
- Vertical scrolling.
- Horizontal page navigation.

Requirements:
- Automatically detect image dimensions and orientation.
- Avoid splitting two-page spreads incorrectly.
- Preserve aspect ratios.
- Support configurable page fitting: fit width, fit height, fit page, and original size.
- Remember the user's preferred reading mode.
- Allow changing modes without losing reading progress.

### 2.2 Advanced Zooming

Provide precise control over comic magnification.

Requirements:
- Mouse-wheel zoom.
- Click-to-zoom.
- Click-and-drag panning.
- Zoom towards the cursor position.
- Double-click to toggle zoom.
- Reset zoom to page fit.
- Smooth zoom transitions.
- Prevent accidental movement outside page boundaries.

Zoom should remain responsive even with high-resolution comic scans.

### 2.3 Page Transitions

Make page navigation feel polished.

Requirements:
- Instant transitions.
- Horizontal slide.
- Fade.
- Optional page-turn animation.
- Configurable transition duration.
- Disable animations option.
- Preload neighbouring pages to minimise loading delays.

Transitions must not introduce noticeable input lag.

### 2.4 Reading Customisation

Allow users to customise the reading experience.

Settings:
- Background colour.
- Page spacing.
- Reading direction.
- Default reading mode.
- Default zoom behaviour.
- Page transition style.
- Interface visibility.
- Fullscreen reading.
- Optional immersive mode with hidden application controls.

Settings should persist between sessions.

### 2.5 Visual Bookshelf

The main library interface.

Requirements:
- Display comics as visual covers.
- Grid and list layouts.
- Sort by title, series, issue number, release date, and recently read.
- Filter by universe, publisher, series, and reading status.
- Display reading progress.
- Highlight currently reading comics.
- Show unread, in-progress, and completed states.
- Support missing-cover placeholders.

The bookshelf should be the primary landing page.

### 2.6 Series Organisation

Organise individual issues into meaningful series.

Requirements:
- Create and edit series.
- Assign issues to series.
- Store series descriptions and cover artwork.
- Track issue numbering.
- Support volumes and publication runs.
- Support alternate covers and special editions.
- Allow series to belong to a universe or publisher.

A series should be able to contain issues imported in any order.

### 2.7 Reading Tracker

Track reading progress at both issue and page level.

Requirements:
- Remember the last-read page.
- Automatically save reading progress.
- Mark issues as unread, in progress, or completed.
- Record when an issue was started and completed.
- Allow manually changing reading status.
- Resume reading from the previous position.
- Track reading history.

The application should not mark an issue as completed merely because the user opened its final page. Completion should be based on a configurable threshold or explicit user action.

### 2.8 Automatic Metadata

Extract and manage comic information.

Supported information:
- Comic title.
- Series.
- Issue number.
- Volume.
- Publisher.
- Release date.
- Writer.
- Artist.
- Cover artist.
- Description.
- Cover image.

Requirements:
- Read embedded comic metadata when available.
- Support manual metadata editing.
- Provide automatic filename-based detection.
- Allow bulk metadata editing.
- Warn about duplicate imports.
- Allow metadata corrections without modifying the original comic file.

Metadata should be stored separately from the source comic files.

### 2.9 Chronological Reading Order

Provide an organised reading order across multiple series.

Requirements:
- Sort issues by publication date.
- Allow manually defined chronological order.
- Support multiple reading orders for the same universe.
- Group issues into events and story arcs.
- Display reading order as a navigable list.
- Allow users to follow a reading order and automatically advance to the next issue.
- Clearly distinguish publication order from chronological/story order.

This is particularly important for universes with multiple concurrent comic series.

### 2.10 Advanced Search

Search the entire library.

Searchable fields:
- Comic title.
- Issue number.
- Series.
- Publisher.
- Writer.
- Artist.
- Release date.
- Characters.
- Story arcs.
- Locations.

Requirements:
- Search filters.
- Sorting.
- Search suggestions.
- Saved searches.
- Search within a particular universe.
- Search across the entire library.

Some fields, such as characters and locations, will become more useful once the universe database is implemented.

### 2.11 Keyboard Shortcuts

Provide configurable keyboard controls.

Default suggestions:
- Left / Right Arrow: Previous / next page.
- Space: Next page.
- Shift + Space: Previous page.
- F: Toggle fullscreen.
- Z: Toggle zoom.
- Escape: Exit fullscreen or close overlays.
- B: Add bookmark.
- Ctrl + F: Search.
- Ctrl + Left / Right: Previous / next issue.

Requirements:
- Customisable bindings.
- Avoid conflicts with text inputs.
- Display available shortcuts in a help panel.

### 2.12 Custom Collections

Allow users to create their own comic groupings.

Examples:
- Favourite issues.
- Sonic IDW.
- Archie Reboot.
- Recommended reading.
- Best Shadow appearances.
- Comics to read later.

Requirements:
- Custom collection names and descriptions.
- Add individual issues or entire series.
- Drag-and-drop ordering.
- Optional custom cover artwork.
- Support overlapping collections.
- Allow collections to be displayed on the bookshelf.

### 2.13 Spoiler Protection

Prevent the application from revealing information the user has not encountered yet.

Requirements:
- Hide character appearances beyond the user's reading progress.
- Hide future story arcs and events.
- Restrict character information to known appearances.
- Hide descriptions containing future plot information.
- Provide an option to reveal protected information manually.
- Allow spoiler protection to be disabled.

Spoiler protection should be based on the user's selected reading order, not simply publication date.

---

## Stage 2 — Universe Database and Advanced Reading

This stage transforms the application from a comic reader into an interconnected comic-universe reference system.

### 2.14 Guided Panel Reading

Allow users to navigate a comic panel by panel rather than page by page.

Requirements:
- Identify individual panels.
- Automatically determine a reasonable reading sequence.
- Highlight the active panel.
- Zoom into the active panel.
- Navigate between panels.
- Support manual correction of panel order.
- Return to normal page reading at any time.

The system should handle unusual layouts, overlapping panels, and irregular reading orders gracefully.

### 2.15 Panel Detection

Automatically identify comic panels using image analysis.

Requirements:
- Analyse imported comic pages.
- Detect panel boundaries.
- Store panel coordinates.
- Associate panels with their original pages.
- Allow manual adjustment of detected boundaries.
- Support irregular panel shapes where practical.
- Cache detection results.

Panel detection should be treated as an independent processing system, so it can run in the background without interrupting reading.

### 2.16 Character Profiles

Create dedicated pages for characters.

Each profile should support:
- Character name.
- Portrait or reference image.
- Description.
- First appearance.
- Known aliases.
- Associated universe.
- Related characters.
- Story arc appearances.
- Issue appearances.

Character information should be spoiler-aware.

### 2.17 Relationship Maps

Visually represent relationships between characters.

Potential relationships:
- Friends.
- Enemies.
- Family.
- Allies.
- Rivals.
- Organisations.
- Team affiliations.

Requirements:
- Interactive node-based graph.
- Select a character to view their connections.
- Navigate directly to related character profiles.
- Display relationship descriptions.
- Support different relationship types.
- Filter connections by universe or reading progress.

Relationships should be editable rather than relying entirely on automatic detection.

### 2.18 Universe Timeline

Provide a visual timeline of events across a comic universe.

Requirements:
- Display major events chronologically.
- Include issue appearances.
- Support overlapping storylines.
- Separate publication date from in-universe chronology.
- Group related events.
- Allow users to navigate from an event to its associated issues.
- Support alternative timelines and continuity branches.

### 2.19 Story Arc Tracking

Create dedicated entries for major story arcs.

Each story arc should contain:
- Title.
- Description.
- Associated issues.
- Participating characters.
- Relevant events.
- Beginning and ending issues.
- Reading status.

Requirements:
- Display all issues belonging to an arc.
- Track completion of individual arcs.
- Allow overlapping arcs.
- Support multi-series storylines.
- Link arcs to the universe timeline.

### 2.20 Location Database

Create a reference database for locations appearing in comics.

Each location should contain:
- Name.
- Description.
- Reference images.
- Associated characters.
- Associated issues.
- Related story arcs.
- First known appearance.

Users should be able to navigate from a location to its associated characters and issues.

### 2.21 Character Appearance Tracker

Track where individual characters appear.

Requirements:
- Record character appearances by issue.
- Optionally record appearances by page and panel.
- Distinguish major appearances from brief cameos.
- Display character appearance history.
- Filter appearances by series, universe, and story arc.
- Link appearances to character profiles.

This feature should eventually integrate with panel detection and optional character recognition.

### 2.22 Multi-device Synchronisation

Synchronise library information and reading progress between devices.

Requirements:
- Sync reading progress.
- Sync metadata.
- Sync bookmarks and collections.
- Sync custom reading orders.
- Sync character and story information.
- Detect conflicting changes.
- Allow local-only operation.
- Support backup and recovery.

Comic image files should not necessarily be synchronised by default. The application should be able to synchronise metadata separately from large comic archives.

---

## Stage 3 — Optional Enhancements

These features are useful but should not delay the completion of the main application.

### 2.23 Statistics Dashboard

Provide an overview of reading activity.

Potential statistics:
- Total issues read.
- Total pages read.
- Total reading time.
- Issues completed this month.
- Reading streaks.
- Most-read series.
- Most-read characters.
- Reading progress by universe.
- Completion percentage of custom collections.

Include simple graphs and visual summaries.

Statistics should be calculated from locally stored reading history.

---

# 3. Additional Suggested Features

These are not part of the selected feature list, but would complement the application.

### 3.1 Bookmarks and Page Notes
- Bookmark specific pages.
- Add private notes to individual pages.
- Add annotations without modifying the original comic.
- Search bookmarks and notes.

### 3.2 Import Management
- Drag and drop comic files.
- Bulk import folders.
- Import CBZ, CBR, and PDF files.
- Detect duplicate issues.
- Display import progress.
- Allow metadata correction during import.

### 3.3 Reading Recommendations
- Recommend the next issue based on a selected reading order.
- Suggest related story arcs.
- Identify missing issues in a series.
- Recommend appearances of a selected character.

### 3.4 Comic Comparison
- View two pages side by side.
- Compare alternate covers.
- Compare different editions or scans.
- Switch between versions without leaving the reader.

### 3.5 Reading Session Management
- Automatically record reading sessions.
- Resume interrupted sessions.
- Display recently opened issues.
- Allow users to manually correct reading progress.

### 3.6 Missing Issue Detection
- Identify gaps in a series.
- Display missing issue numbers.
- Distinguish intentionally skipped issues from missing imports.
- Allow users to mark issues as unavailable or not required.

### 3.7 Custom Themes
- Light and dark themes.
- Custom accent colours.
- Adjustable interface density.
- Optional universe-specific themes.

---

# 4. Technical Architecture

The application should be built around a modular architecture.

## Recommended technology stack

- **Desktop framework:** Tauri.
- **Frontend:** React + TypeScript.
- **Styling:** Tailwind CSS.
- **Database:** SQLite.
- **Comic archives:** CBZ, CBR, PDF, and image folders.
- **Image processing:** Dedicated image-processing module.
- **Panel detection:** Separate optional computer-vision module.
- **Synchronisation:** Future service using a secure sync protocol.

The frontend should communicate with the local application backend through clearly defined interfaces.

## Suggested application modules

1. **Reader:** Page rendering, zooming, navigation, transitions, and reading modes.
2. **Library:** Importing, bookshelf, series, collections, and file management.
3. **Metadata:** Comic information, issue relationships, and search indexing.
4. **Reading Progress:** Bookmarks, history, completion, and reading orders.
5. **Universe Database:** Characters, locations, relationships, story arcs, and timelines.
6. **Image Processing:** Thumbnail generation, caching, panel detection, and image optimisation.
7. **Synchronisation:** Future multi-device data synchronisation.

## Data storage principles

- Keep original comic archives untouched.
- Store application metadata in SQLite.
- Store generated thumbnails and image caches separately.
- Store panel detection results separately from original pages.
- Keep reading progress independent of comic file paths wherever possible.
- Support database backups and restoration.

---

# 5. Suggested Development Order

| Milestone | Deliverable |
|---|---|
| 1 | Application shell and navigation |
| 2 | Comic importing and metadata |
| 3 | Visual bookshelf and series management |
| 4 | Core comic reader |
| 5 | Reading modes, zoom, transitions, and customisation |
| 6 | Reading progress and keyboard shortcuts |
| 7 | Search, collections, and reading orders |
| 8 | Spoiler protection |
| 9 | Panel detection and guided reading |
| 10 | Character, location, and story arc database |
| 11 | Relationships and universe timeline |
| 12 | Character appearance tracking |
| 13 | Multi-device synchronisation |
| 14 | Statistics dashboard and polish |

---

# 6. Definition of a Successful First Release

The first release should be considered complete when a user can:

1. Import a collection of comic files.
2. Have the application identify and organise their issues.
3. Browse a visual bookshelf.
4. Open any issue in a responsive comic reader.
5. Switch between reading modes.
6. Zoom, pan, and navigate comfortably.
7. Customise the reading experience.
8. Automatically save and resume reading progress.
9. Search and organise their library.
10. Follow a custom chronological reading order.
11. Protect themselves from spoilers.
12. Use keyboard shortcuts to control the reader.

The first release does not need AI, character recognition, panel detection, or synchronisation. Those systems should be added after the core application is stable.

**The central goal is to make reading and organising comics significantly better than using a folder of CBZ files and a conventional image viewer.**