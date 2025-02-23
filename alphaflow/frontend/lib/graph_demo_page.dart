import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:node_flow/flutter_graph_editor.dart';

class GraphDemoPage extends ConsumerStatefulWidget {
  // 如果你想从外部传 config, 写:
  // final GraphConfig config;
  // const GraphDemoPage({super.key, required this.config});

  const GraphDemoPage({super.key});

  @override
  ConsumerState<GraphDemoPage> createState() => _GraphDemoPageState();
}

class _GraphDemoPageState extends ConsumerState<GraphDemoPage> {
  late final GraphConfig _config;

  @override
  void initState() {
    super.initState();

    // 创建 config (若想外部传入, 就 widget.config)
    _config = GraphConfig(
      canvasConfig:
          const CanvasConfig(viewportWidth: 1024, viewportHeight: 768),
      nodeConfig: const NodeConfig(),
      edgeConfig: const EdgeConfig(),
      interactionConfig: InteractionConfig(),
    );

    // 延迟添加节点
    WidgetsBinding.instance.addPostFrameCallback((_) => _initTestNodes());
  }

  void _initTestNodes() {
    final nodesNotifier = ref.read(nodesProvider.notifier);

    nodesNotifier.addNode(NodeModel(
      id: 'node-default',
      title: '默认节点',
      x: 100,
      y: 100,
      width: _config.nodeConfig.defaultNodeWidth,
      height: _config.nodeConfig.defaultNodeHeight,
      color: Colors.blue,
      type: 'default',
      // anchors...
      anchors: [
        AnchorModel(id: 'left', position: Position.left),
        AnchorModel(id: 'right', position: Position.right),
      ],
      dragMode: DragMode.full,
    ));
    // 其余节点(省略)...
  }

  void _addRandomNode() {
    final rand = math.Random();
    final newId = 'node-${DateTime.now().millisecondsSinceEpoch}';
    final newX = (_config.canvasConfig.viewportWidth -
            _config.nodeConfig.defaultNodeWidth) *
        rand.nextDouble();
    final newY = (_config.canvasConfig.viewportHeight -
            _config.nodeConfig.defaultNodeHeight) *
        rand.nextDouble();
    final color = Color.fromRGBO(
        rand.nextInt(256), rand.nextInt(256), rand.nextInt(256), 1);

    ref.read(nodesProvider.notifier).addNode(NodeModel(
          id: newId,
          title: '随机节点',
          x: newX,
          y: newY,
          width: _config.nodeConfig.defaultNodeWidth,
          height: _config.nodeConfig.defaultNodeHeight,
          color: color,
          type: 'default',
          anchors: [
            AnchorModel(id: 'left', position: Position.left),
            AnchorModel(id: 'right', position: Position.right),
          ],
          dragMode: DragMode.full,
        ));
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Node Flow Demo'),
      ),
      body: Center(
        child: GraphCanvas(
          config: _config,
          plugins: [
            Positioned(
              left: 16,
              bottom: 16,
              child: Card(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    ZoomAndFitViewPlugin(config: _config),
                    const Divider(height: 1),
                    const Divider(height: 1),
                    const AutoLayoutPlugin(),
                    const Divider(height: 1),
                    const BackgroundStylePlugin(),
                    const Divider(height: 1),
                    const ThemeTogglePlugin(),
                    const Divider(height: 1),
                    ModeTogglePlugin(
                      interactionConfig: _config.interactionConfig,
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: _addRandomNode,
        icon: const Icon(Icons.add),
        label: const Text('添加节点'),
      ),
    );
  }
}
