import type { Category } from "../../../generated/Category";

// Each category with its full path for pickers and lists, sorted by path.
// Example: "Dairy › Milk". A category whose parent is missing shows on its own.
export function categoryPaths(
  categories: Category[],
): { id: string; path: string; depth: number }[] {
  const byId = new Map(categories.map((each) => [each.id, each]));
  const pathOf = (category: Category): string[] => {
    const names = [category.name];
    let parent = category.parent_id ? byId.get(category.parent_id) : undefined;
    // A broken tree can't loop for ever: stop after as many steps as there are categories.
    for (let step = 0; parent && step < categories.length; step += 1) {
      names.unshift(parent.name);
      parent = parent.parent_id ? byId.get(parent.parent_id) : undefined;
    }
    return names;
  };
  return categories
    .map((each) => {
      const names = pathOf(each);
      return { id: each.id, path: names.join(" › "), depth: names.length - 1 };
    })
    .sort((a, b) => a.path.localeCompare(b.path));
}
