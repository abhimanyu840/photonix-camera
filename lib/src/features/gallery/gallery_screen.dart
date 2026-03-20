import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../providers/providers.dart';
import '../../models/camera_state.dart';
import '../../shared/theme/app_theme.dart';

class GalleryScreen extends ConsumerWidget {
  const GalleryScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final galleryAsync = ref.watch(galleryProvider);

    return Scaffold(
      backgroundColor: PhotonixColors.background,
      appBar: AppBar(
        title: const Text('Gallery'),
        actions: [
          Semantics(
            label: 'Select photos',
            button: true,
            child: IconButton(
              icon: const Icon(Icons.select_all_outlined),
              onPressed: () {}, // multi-select future feature
            ),
          ),
        ],
      ),
      body: galleryAsync.when(
        loading: () => const Center(
          child: CircularProgressIndicator(color: PhotonixColors.accent),
        ),
        error: (e, _) => Center(
          child: Text(
            'Error loading gallery: $e',
            style: const TextStyle(color: PhotonixColors.error),
          ),
        ),
        data: (photos) {
          if (photos.isEmpty) {
            return Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(
                    Icons.photo_library_outlined,
                    color: PhotonixColors.textTertiary,
                    size: 48,
                  ),
                  const SizedBox(height: 16),
                  const Text(
                    'No photos yet',
                    style: TextStyle(
                      color: PhotonixColors.textPrimary,
                      fontSize: 17,
                    ),
                  ),
                  const SizedBox(height: 8),
                  const Text(
                    'Tap the shutter to capture your first photo',
                    style: TextStyle(
                      color: PhotonixColors.textSecondary,
                      fontSize: 13,
                    ),
                    textAlign: TextAlign.center,
                  ),
                ],
              ),
            );
          }

          return GridView.builder(
            padding: const EdgeInsets.all(2),
            gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: 3,
              crossAxisSpacing: 2,
              mainAxisSpacing: 2,
            ),
            itemCount: photos.length,
            itemBuilder: (context, index) {
              final photo = photos[index];
              return _PhotoGridItem(
                photo: photo,
                onTap: () => context.push('/photo/${photo.id}'),
              );
            },
          );
        },
      ),
    );
  }
}

class _PhotoGridItem extends StatelessWidget {
  final PhotoEntry photo;
  final VoidCallback onTap;

  const _PhotoGridItem({required this.photo, required this.onTap});

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label:
          'Photo taken ${_formatDate(photo.capturedAt)}, '
          '${photo.sceneType ?? "standard"} scene, '
          '${photo.processingTimeMs}ms processing time',
      button: true,
      child: GestureDetector(
        onTap: onTap,
        child: Stack(
          fit: StackFit.expand,
          children: [
            // Placeholder thumbnail (real image from file in production)
            Container(
              color: PhotonixColors.surface,
              child: const Icon(
                Icons.image_outlined,
                color: PhotonixColors.textTertiary,
                size: 24,
              ),
            ),

            // Metadata overlay at bottom
            Positioned(
              bottom: 0,
              left: 0,
              right: 0,
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 3),
                decoration: const BoxDecoration(
                  gradient: LinearGradient(
                    begin: Alignment.bottomCenter,
                    end: Alignment.topCenter,
                    colors: [Colors.black87, Colors.transparent],
                  ),
                ),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    if (photo.sceneType != null)
                      Text(
                        photo.sceneType!.toUpperCase(),
                        style: const TextStyle(
                          color: PhotonixColors.accent,
                          fontSize: 7,
                          fontWeight: FontWeight.w600,
                          letterSpacing: 0.5,
                        ),
                      ),
                    Text(
                      '${photo.processingTimeMs}ms',
                      style: const TextStyle(
                        color: Colors.white54,
                        fontSize: 7,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  String _formatDate(DateTime dt) =>
      '${dt.day}/${dt.month}/${dt.year} ${dt.hour}:${dt.minute.toString().padLeft(2, '0')}';
}
