import 'package:flutter/material.dart';

class WorkflowPage extends StatelessWidget {
  final Map<String, dynamic> userJson;
  const WorkflowPage({super.key, required this.userJson});

  @override
  Widget build(BuildContext context) {
    final userEmail = userJson['email'] ?? 'NoEmail';
    return Scaffold(
      appBar: AppBar(title: const Text("Workflow Page")),
      body: Center(
        child: Text("Welcome, $userEmail! Here is the workflow UI..."),
      ),
    );
  }
}
