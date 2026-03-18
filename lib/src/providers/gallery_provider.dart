import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../models/camera_state.dart';

/// In-memory gallery — persisted to disk in P10.
/// AsyncNotifier because future phases load from filesystem.
class GalleryNotifier extends AsyncNotifier<List<PhotoEntry>> {
  @override
  Future<List<PhotoEntry>> build() async {
    // P10: load from app documents directory
    // For now return empty list
    return [];
  }

  void addPhoto(PhotoEntry photo) {
    final current = state.valueOrNull ?? [];
    state = AsyncData([photo, ...current]);
  }

  void removePhoto(String id) {
    final current = state.valueOrNull ?? [];
    state = AsyncData(current.where((p) => p.id != id).toList());
  }
}

final galleryProvider =
    AsyncNotifierProvider<GalleryNotifier, List<PhotoEntry>>(
  GalleryNotifier.new,
);