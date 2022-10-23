export function formatTitle(
  resultTitle: string, 
  useBreadcrumb: boolean,
  relativeLink: string,
) {
  if (!resultTitle || useBreadcrumb) {
    // HTML files: remove the extension
    // PDF: <...breadcumbs...> (PDF)
    const breadCrumbed = relativeLink.split('/')
      .map((component) => {
        /*
            Separate on spaces, underscores, dashes.
            Then assume each sub-component is in camelCase,
            and try to convert to title case.
          */
        return component.split(/[\s_-]+/g)
          .map((text) => text.replace(/([a-z])([A-Z])/g, '$1 $2'))
          .map((text) => text.charAt(0).toUpperCase() + text.slice(1))
          .join(' ');
      })
      .join(' » ');
    const breadCrumbsAndExt = breadCrumbed.split('.');

    let ext = breadCrumbsAndExt.pop().toUpperCase();
    if (ext === 'HTML') {
      ext = '';
    } else if (ext === 'PDF') {
      ext = ' (PDF)';
    } else {
      ext = '.' + ext;
    }

    return breadCrumbsAndExt.join('.') + ext;
  }

  return resultTitle;
}
