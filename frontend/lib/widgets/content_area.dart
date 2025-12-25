import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:markdown/markdown.dart' as md;
import 'package:notes/show.dart';
import 'package:notes/state/app_state_scope.dart';

class ContentArea extends StatelessWidget {
  const ContentArea({super.key});

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    final openDocIds = appState.openDocumentIds;
    final activeDocId = appState.activeDocumentId;

    if (openDocIds.isEmpty) {
      return Focus(
        autofocus: true,
        child: Builder(
          builder: (context) {
            return GestureDetector(
              behavior: HitTestBehavior.translucent,
              onTap: () =>
                  FocusScope.of(context).requestFocus(Focus.of(context)),
              child: Center(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    _buildShortcutRow(
                      showSearchPopup,
                      context,
                      'Search',
                      'Ctrl + P',
                      Icons.search,
                    ),
                    _buildShortcutRow(
                      showConfigurationPopup,
                      context,
                      'Configurations',
                      'Ctrl + C',
                      Icons.settings,
                    ),
                  ],
                ),
              ),
            );
          },
        ),
      );
    }

    return Column(
      children: [
        // Tab Bar
        SizedBox(
          height: 48,
          child: ListView.builder(
            scrollDirection: Axis.horizontal,
            itemCount: openDocIds.length,
            itemBuilder: (context, index) {
              final docId = openDocIds[index];
              final doc = appState.documentById[docId];
              final isActive = docId == activeDocId;

              return InkWell(
                onTap: () => appState.setActiveDocument(docId),
                child: Container(
                  width: 150, // Fixed width for tabs
                  padding: const EdgeInsets.symmetric(horizontal: 8),
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    color: isActive
                        ? Theme.of(context).colorScheme.surface
                        : Theme.of(context).colorScheme.surfaceContainerHighest,
                    border: Border(
                      right: BorderSide(color: Theme.of(context).dividerColor),
                      top: isActive
                          ? BorderSide(
                              color: Theme.of(context).primaryColor,
                              width: 2,
                            )
                          : BorderSide.none,
                    ),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Expanded(
                        child: Text(
                          doc?.title ?? 'Loading...',
                          overflow: TextOverflow.ellipsis,
                          maxLines: 1,
                        ),
                      ),
                      const SizedBox(width: 4),
                      InkWell(
                        onTap: () => appState.closeDocument(docId),
                        child: const Icon(Icons.close, size: 16),
                      ),
                    ],
                  ),
                ),
              );
            },
          ),
        ),
        // Content
        Expanded(
          child: activeDocId != null
              ? DocumentEditor(
                  key: ValueKey(activeDocId),
                  documentId: activeDocId,
                )
              : const SizedBox(),
        ),
      ],
    );
  }

  Widget _buildShortcutRow(
    void Function(BuildContext context) popupFunction,
    BuildContext context,
    String label,
    String shortcut,
    IconData icon,
  ) {
    void _popupFunction() => popupFunction(context);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          GestureDetector(
            onTap: _popupFunction,
            child: Row(
              spacing: 12,
              children: [
                Icon(
                  icon,
                  size: 20,
                  color: Theme.of(context).colorScheme.secondary,
                ),
                Text(
                  label,
                  style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                    color: Theme.of(context).colorScheme.secondary,
                  ),
                ),
                Text(
                  shortcut,
                  style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: Theme.of(context).colorScheme.onSurface,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class DocumentEditor extends StatefulWidget {
  final String documentId;
  const DocumentEditor({super.key, required this.documentId});

  @override
  State<DocumentEditor> createState() => _DocumentEditorState();
}

class _DocumentEditorState extends State<DocumentEditor> {
  late MarkdownTextEditingController _controller;
  final FocusNode _focusNode = FocusNode();
  late ScrollController _editorScrollController;

  @override
  void initState() {
    super.initState();
    _controller = MarkdownTextEditingController();
    _editorScrollController = ScrollController();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _updateContent();
  }

  @override
  void didUpdateWidget(DocumentEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.documentId != widget.documentId) {
      _updateContent();
    }
  }

  void _updateContent() {
    final appState = AppStateScope.of(context);
    final content = appState.documentContentCache[widget.documentId] ?? '';
    bool contentUpdated = false;

    // Only update if drastically different to avoid cursor jump?
    // Or simpler: only update if controller is empty (initial load).
    // If user is typing, we shouldn't overwrite unless it's a remote update.
    // For now, let's assume single user editing locally.
    if (_controller.text != content &&
        content.isNotEmpty &&
        _controller.text.isEmpty) {
      _controller.text = content;
      contentUpdated = true;
    } else if (content.isNotEmpty && _controller.text != content) {
      // Logic for remote update conflicts could go here.
      // For now, respect local edits over cache unless cache is empty?
      // Actually, if we switch tabs, we reload from cache.
      // So if cache is updated by us, it's fine.
      _controller.text = content;
      contentUpdated = true;
    }

    if (contentUpdated || content.isNotEmpty) {
      _checkHighlight();
    }
  }

  void _checkHighlight() {
    final appState = AppStateScope.of(context);
    final highlight = appState.searchHighlights[widget.documentId];
    if (highlight != null && _controller.text.isNotEmpty) {
      final highlightText = highlight.text;
      int index = -1;
      int length = highlightText.length;

      // Try chunk offset
      if (highlight.chunkId != null) {
        final offsets = appState.documentChunkOffsets[widget.documentId];
        if (offsets != null && offsets.containsKey(highlight.chunkId)) {
          index = offsets[highlight.chunkId]!;
        }
      }

      if (index == -1) {
        index = _controller.text.indexOf(highlightText);
      }

      // Fallback 1: Try matching the first 50 characters
      // This handles cases where the chunk is long and may have minor tail differences
      if (index == -1 && highlightText.length > 50) {
        final shortHighlight = highlightText.substring(0, 50);
        index = _controller.text.indexOf(shortHighlight);
        if (index != -1) {
          length = 50;
        }
      }

      // Fallback 2: Try matching segments to avoid newline mismatch (\r\n vs \n)
      if (index == -1) {
        final parts = highlightText.split(RegExp(r'[\r\n]+'));
        String? bestPart;
        // Find the first significant part
        for (final part in parts) {
          if (part.length > 15) {
            bestPart = part;
            break;
          }
        }
        // If no long part found, take the longest one available
        if (bestPart == null && parts.isNotEmpty) {
          // Create a copy to sort
          final sortedParts = List<String>.from(parts)
            ..sort((a, b) => b.length.compareTo(a.length));
          if (sortedParts.isNotEmpty) bestPart = sortedParts.first;
        }

        if (bestPart != null && bestPart.isNotEmpty) {
          index = _controller.text.indexOf(bestPart);
          if (index != -1) {
            length = bestPart.length;
          }
        }
      }

      if (index != -1) {
        // Clear highlight from state so we don't jump again
        appState.searchHighlights.remove(widget.documentId);

        // Calculate selection
        final selection = TextSelection(
          baseOffset: index,
          extentOffset: index + length,
        );

        _controller.selection = selection;

        // Ensure focus and scroll
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) {
            _focusNode.requestFocus();
          }
        });
      }
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose();
    _editorScrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final appState = AppStateScope.of(context);
    return Column(
      children: [
        // Editor
        Expanded(
          child: TextField(
            controller: _controller,
            scrollController: _editorScrollController,
            focusNode: _focusNode,
            maxLines: null,
            expands: true,
            decoration: const InputDecoration(
              border: InputBorder.none,
              contentPadding: EdgeInsets.all(16),
              hintText: 'Start typing...',
            ),
            onChanged: (value) {
              appState.updateDocumentDraft(widget.documentId, value);
            },
          ),
        ),
      ],
    );
  }
}

class MarkdownTextEditingController extends TextEditingController {
  List<MarkdownSpanData>? _cachedSpans;
  String? _cachedText;
  Timer? _debounceTimer;
  bool _isParsing = false;

  @override
  TextSpan buildTextSpan({
    required BuildContext context,
    TextStyle? style,
    required bool withComposing,
  }) {
    // 1. If text matches cache exactly, use it.
    if (_cachedText == text && _cachedSpans != null) {
      return _buildSpansFromData(context, style, _cachedSpans!);
    }

    // 2. Schedule a fresh parse (debounced)
    _scheduleParse();

    // 3. Try to shift existing spans to match the new text
    // This prevents "flashing" to plain text while waiting for the parser
    if (_cachedText != null && _cachedSpans != null) {
      final shiftedSpans = _shiftSpans(_cachedText!, text, _cachedSpans!);
      if (shiftedSpans != null) {
        // Temporarily update cache to the shifted version so we don't re-calculate every frame
        // (Note: _cachedText remains the *old* confirmed text for the isolate logic,
        // but for rendering we can use a temporary "speculative" cache if we wanted,
        // but simpler is just to return the result).
        return _buildSpansFromData(context, style, shiftedSpans);
      }
    }

    // 4. Fallback if shifting failed (e.g. massive change)
    return _buildSimpleSpans(context, style);
  }

  // Helper to build TextSpans from our data objects
  TextSpan _buildSpansFromData(
    BuildContext context,
    TextStyle? style,
    List<MarkdownSpanData> spanData,
  ) {
    final List<TextSpan> children = [];
    int currentIndex = 0;

    // Sort just in case, though they should be sorted
    // spanData.sort((a, b) => a.start.compareTo(b.start));

    for (final span in spanData) {
      // Safety check for bounds
      if (span.start >= text.length) break;
      int end = span.end > text.length ? text.length : span.end;
      if (end < span.start) continue;

      if (span.start > currentIndex) {
        children.add(
          TextSpan(
            text: text.substring(currentIndex, span.start),
            style: style?.copyWith(color: Colors.grey),
          ),
        );
      }

      TextStyle? spanStyle = style;
      switch (span.type) {
        case 'h1':
          spanStyle = style?.copyWith(
            fontSize: 24,
            fontWeight: FontWeight.bold,
            color: Theme.of(context).colorScheme.primary,
          );
          break;
        case 'h2':
          spanStyle = style?.copyWith(
            fontSize: 22,
            fontWeight: FontWeight.bold,
            color: Theme.of(context).colorScheme.primary,
          );
          break;
        case 'h3':
          spanStyle = style?.copyWith(
            fontSize: 20,
            fontWeight: FontWeight.bold,
            color: Theme.of(context).colorScheme.primary,
          );
          break;
        case 'h4':
          spanStyle = style?.copyWith(
            fontSize: 18,
            fontWeight: FontWeight.bold,
            color: Theme.of(context).colorScheme.primary,
          );
          break;
        case 'h5':
          spanStyle = style?.copyWith(
            fontSize: 16,
            fontWeight: FontWeight.bold,
            color: Theme.of(context).colorScheme.primary,
          );
          break;
        case 'h6':
          spanStyle = style?.copyWith(
            fontSize: 14,
            fontWeight: FontWeight.bold,
            color: Theme.of(context).colorScheme.primary,
          );
          break;
        case 'strong':
          spanStyle = style?.copyWith(fontWeight: FontWeight.bold);
          break;
        case 'em':
          spanStyle = style?.copyWith(fontStyle: FontStyle.italic);
          break;
        case 'code':
        case 'pre':
          spanStyle = style?.copyWith(
            fontFamily: 'monospace',
            backgroundColor: Theme.of(
              context,
            ).colorScheme.surfaceContainerHighest,
          );
          break;
        case 'a':
          spanStyle = style?.copyWith(
            color: Colors.blue,
            decoration: TextDecoration.underline,
          );
          break;
      }

      children.add(
        TextSpan(text: text.substring(span.start, end), style: spanStyle),
      );

      currentIndex = end;
    }

    if (currentIndex < text.length) {
      children.add(
        TextSpan(
          text: text.substring(currentIndex),
          style: style?.copyWith(color: Colors.grey),
        ),
      );
    }

    return TextSpan(style: style, children: children);
  }

  // Tries to map old spans to new text based on simple diff
  List<MarkdownSpanData>? _shiftSpans(
    String oldText,
    String newText,
    List<MarkdownSpanData> oldSpans,
  ) {
    // Only handle simple insertion/deletion
    if ((newText.length - oldText.length).abs() > 10)
      return null; // Too complex/large change

    // Find first difference
    int start = 0;
    int minLen = oldText.length < newText.length
        ? oldText.length
        : newText.length;
    while (start < minLen &&
        oldText.codeUnitAt(start) == newText.codeUnitAt(start)) {
      start++;
    }

    // If identical (shouldn't happen here), return old
    if (start == minLen && oldText.length == newText.length) return oldSpans;

    int delta = newText.length - oldText.length;

    // Construct new spans
    List<MarkdownSpanData> newSpans = [];
    for (final span in oldSpans) {
      int newStart = span.start;
      int newEnd = span.end;

      // Case 1: Change happened before this span
      if (start < span.start) {
        newStart += delta;
        newEnd += delta;
      }
      // Case 2: Change happened inside this span
      else if (start >= span.start && start < span.end) {
        newEnd += delta;
      }
      // Case 3: Change happened after this span -> no change

      // Validation
      if (newStart < 0) newStart = 0;
      if (newEnd < newStart) newEnd = newStart; // Collapsed

      newSpans.add(MarkdownSpanData(newStart, newEnd, span.type));
    }
    return newSpans;
  }

  TextSpan _buildSimpleSpans(BuildContext context, TextStyle? style) {
    // A very lightweight regex pass for headers and bold only
    // to prevent the UI from looking completely plain while typing
    final children = <TextSpan>[];
    text.splitMapJoin(
      RegExp(r'(^\s*#{1,6}\s+.*$)|(\*\*.*?\*\*)', multiLine: true),
      onMatch: (m) {
        final match = m[0]!;
        TextStyle? newStyle = style;
        if (match.trim().startsWith('#')) {
          newStyle = style?.copyWith(
            fontWeight: FontWeight.bold,
            color: Theme.of(context).colorScheme.primary,
          );
        } else if (match.startsWith('**')) {
          newStyle = style?.copyWith(fontWeight: FontWeight.bold);
        }
        children.add(TextSpan(text: match, style: newStyle));
        return '';
      },
      onNonMatch: (n) {
        children.add(TextSpan(text: n, style: style));
        return '';
      },
    );
    return TextSpan(style: style, children: children);
  }

  void _scheduleParse() {
    if (_debounceTimer?.isActive ?? false) _debounceTimer!.cancel();

    _debounceTimer = Timer(const Duration(milliseconds: 100), () async {
      final currentText = text;
      if (currentText == _cachedText) return;

      _isParsing = true;
      try {
        final spans = await compute(_parseMarkdownWorker, currentText);

        // Check if text has changed while we were parsing
        if (currentText == text) {
          _cachedText = currentText;
          _cachedSpans = spans;
          notifyListeners(); // Trigger rebuild
        }
      } catch (e) {
        debugPrint('Markdown parsing failed: $e');
      } finally {
        _isParsing = false;
      }
    });
  }
}

// Data class for passing span info back from isolate
class MarkdownSpanData {
  final int start;
  final int end;
  final String type;

  MarkdownSpanData(this.start, this.end, this.type);
}

// Top-level function for isolate
List<MarkdownSpanData> _parseMarkdownWorker(String text) {
  final document = md.Document(extensionSet: md.ExtensionSet.gitHubFlavored);
  final lines = text.split('\n');
  final nodes = document.parseLines(lines);

  final List<MarkdownSpanData> spans = [];
  int currentIndex = 0;

  void visit(List<md.Node> nodes) {
    for (final node in nodes) {
      if (node is md.Element) {
        String? type;
        if ([
          'h1',
          'h2',
          'h3',
          'h4',
          'h5',
          'h6',
          'strong',
          'em',
          'code',
          'pre',
          'a',
        ].contains(node.tag)) {
          type = node.tag;
        }

        if (node.children != null) {
          visit(node.children!);
        }

        // For block elements or elements that wrap content, we rely on children visiting
        // to find the actual text.
        // However, for code blocks (pre > code), the text might be direct child.
      } else if (node is md.Text) {
        final content = node.text;
        final escaped = RegExp.escape(content);
        final patternStr = escaped.replaceAll(' ', r'\s+');
        final pattern = RegExp(patternStr);

        final match = pattern.firstMatch(text.substring(currentIndex));

        if (match != null) {
          final matchStart = currentIndex + match.start;
          final matchEnd = currentIndex + match.end;

          // If we are inside an element of interest, record the span
          // But wait, the 'visit' recursion doesn't pass down the 'active type'.
          // We need to know which style applies to this text node.
          // Let's refactor visit to pass parent type.
        }
      }
    }
  }

  // Re-implement visit to capture type context
  void visitWithContext(List<md.Node> nodes, String? parentType) {
    for (final node in nodes) {
      if (node is md.Element) {
        String? newType = parentType;
        // Only override if it's a styling tag
        if ([
          'h1',
          'h2',
          'h3',
          'h4',
          'h5',
          'h6',
          'strong',
          'em',
          'code',
          'pre',
          'a',
        ].contains(node.tag)) {
          newType = node.tag;
        }

        if (node.children != null) {
          visitWithContext(node.children!, newType);
        }
      } else if (node is md.Text) {
        if (parentType == null) continue; // No style needed

        final content = node.text;

        // Improved matching: Use allMatches to scan efficiently without substring allocation
        final escaped = RegExp.escape(content);
        final patternStr = escaped.replaceAll(' ', r'\s+');
        final pattern = RegExp(patternStr);

        // Find the first match starting at or after currentIndex
        final matches = pattern.allMatches(text, currentIndex);
        if (matches.isNotEmpty) {
          final match = matches.first;
          // Sanity check: if the match is too far ahead, it might be a wrong match for repeated text
          // But since we traverse in order, it should be the next occurrence.
          // However, if we skipped some whitespace, match.start might be > currentIndex.

          if (match.start >= currentIndex) {
            spans.add(MarkdownSpanData(match.start, match.end, parentType));
            currentIndex = match.end;
          }
        }
      }
    }
  }

  visitWithContext(nodes, null);
  return spans;
}
