import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../providers/providers.dart';
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
        backgroundColor: PhotonixColors.background,
      ),
      body: galleryAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(
          child: Text(
            'Error: $e',
            style: const TextStyle(color: PhotonixColors.error),
          ),
        ),
        data: (photos) => photos.isEmpty
            ? const Center(
                child: Text(
                  'No photos yet.\nTap the shutter to capture.',
                  textAlign: TextAlign.center,
                  style: TextStyle(color: PhotonixColors.textSecondary),
                ),
              )
            : GridView.builder(
                padding: const EdgeInsets.all(2),
                gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
                  crossAxisCount: 3,
                  crossAxisSpacing: 2,
                  mainAxisSpacing: 2,
                ),
                itemCount: photos.length,
                itemBuilder: (context, index) {
                  final photo = photos[index];
                  return Container(
                    color: PhotonixColors.surface,
                    child: Center(
                      child: Text(
                        photo.id,
                        style: const TextStyle(
                          color: PhotonixColors.textTertiary,
                          fontSize: 10,
                        ),
                      ),
                    ),
                  );
                },
              ),
      ),
    );
  }
}
